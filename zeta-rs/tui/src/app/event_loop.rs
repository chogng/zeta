use super::ActiveConversation;
use super::App;
use super::AppCommand;
use super::AppEvent;
use super::Status;
use super::frame;
use super::slash_command_registry;
use crate::TuiError;
use crate::TuiExit;
use crate::TuiOptions;
use crate::client;
use crate::components::composer::ComposerSubmission;
use crate::features::skills;
use crate::features::thread::ActiveTurnUpdate;
use crate::features::thread::ThreadRequestScope;
use crate::features::thread::ThreadSubscription;
use crate::features::thread::ThreadSwitch;
use crate::features::thread::evaluate_active_turn;
use crate::features::thread::interrupt_turn as request_interrupt_turn;
use crate::features::thread::read_thread;
use crate::features::thread::submit_prompt as request_submit_prompt;
use crate::features::workspace_files::FileSearchManager;
use crate::host;
use crate::terminal;
use crossterm::event::Event;
use crossterm::event::KeyEventKind;
use crossterm::event::MouseButton;
use crossterm::event::MouseEventKind;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::JsonRpcTransport;
#[cfg(test)]
use zeta_protocol::Turn;
use zeta_protocol::TurnId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ThreadRefresh {
    Applied { sequence: u64 },
    Failed,
}

pub(crate) fn run(mut session: AppServerSession, options: TuiOptions) -> Result<TuiExit, TuiError> {
    let result = run_session(&mut session, options);
    let shutdown = session.shutdown();
    match (result, shutdown) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error.into()),
        (Ok(exit), Ok(())) => Ok(exit),
    }
}

fn run_session(session: &mut AppServerSession, options: TuiOptions) -> Result<TuiExit, TuiError> {
    crate::ui::configure();
    let mut client = session.client();
    let events = session.take_events()?;
    let TuiOptions {
        thread_title,
        workspace_root,
    } = options;
    let slash_commands = slash_command_registry(&client.initialization()?.slash_commands)?;
    let mut conversation = ActiveConversation::start(&mut client, thread_title)?;
    let mut active_turn = None;
    let (mut thread_subscription, initial_thread) = ThreadSubscription::start(
        &mut client,
        conversation.session_id(),
        conversation.thread_id(),
    )?;
    conversation.set_thread_sequence(initial_thread.sequence);
    let mut terminal = terminal::TerminalSession::open()?;
    let mut file_search = FileSearchManager::new(workspace_root.clone());
    let mut app = App::for_workspace_with_slash_commands(&workspace_root, slash_commands);
    app.update(AppEvent::ThreadSnapshotReceived(initial_thread));
    if let Ok(config) = client.read_config() {
        app.update(AppEvent::ConfigSnapshotReceived(config));
    }

    let pump = client::EventPump::start(events)?;
    if let Err(error) = terminal.draw(|terminal_frame| frame::draw(terminal_frame, &app)) {
        let _ = pump.shutdown();
        return Err(error.into());
    }
    let result = (|| {
        loop {
            let action = match pump.recv()? {
                client::RuntimeEvent::Client(event) => {
                    refresh_server_event(
                        event,
                        &mut client,
                        &mut conversation,
                        &mut active_turn,
                        &mut thread_subscription,
                        &mut app,
                    );
                    None
                }
                client::RuntimeEvent::Tick => None,
                client::RuntimeEvent::TerminationRequested => {
                    return Ok(TuiExit::TerminationRequested);
                }
                client::RuntimeEvent::TerminalFailed(error) => return Err(error.into()),
                client::RuntimeEvent::Terminal(event) => match event {
                    Event::Key(key) if key.kind != KeyEventKind::Release => app.handle_key(key),
                    Event::Mouse(mouse)
                        if mouse.kind == MouseEventKind::Down(MouseButton::Left) =>
                    {
                        let terminal_area = terminal.area()?;
                        if let Some(index) =
                            frame::mention_index_at(&app, terminal_area, mouse.column, mouse.row)
                        {
                            app.activate_mention(index);
                            None
                        } else {
                            frame::slash_command_index_at(
                                &app,
                                terminal_area,
                                mouse.column,
                                mouse.row,
                            )
                            .and_then(|index| app.activate_slash_command(index))
                        }
                    }
                    Event::Paste(text) => {
                        app.handle_paste(text);
                        None
                    }
                    _ => None,
                },
            };

            sync_file_search_query(&app, &mut file_search);
            for snapshot in file_search.poll() {
                app.update(AppEvent::FileSearchSnapshotReceived(snapshot));
            }

            if let Some(action) = action {
                match action {
                    AppCommand::ExecuteProductCommand(invocation) => {
                        let previous_thread_id = conversation.thread_id().clone();
                        conversation.execute(&mut client, invocation, &mut app);
                        if conversation.thread_id() != &previous_thread_id {
                            active_turn = None;
                            match thread_subscription.switch(
                                &mut client,
                                conversation.session_id(),
                                conversation.thread_id(),
                            ) {
                                Ok(ThreadSwitch::Complete { snapshot }) => {
                                    conversation.set_thread_sequence(snapshot.sequence);
                                    app.update(AppEvent::ThreadSnapshotReceived(snapshot));
                                }
                                Ok(ThreadSwitch::StaleSubscription { snapshot, error }) => {
                                    conversation.set_thread_sequence(snapshot.sequence);
                                    app.update(AppEvent::ThreadSnapshotReceived(snapshot));
                                    app.update(AppEvent::FailureReported(format!(
                                        "switched Thread, but could not unsubscribe the previous \
                                     Thread: {error}"
                                    )));
                                }
                                Err(error) => {
                                    app.update(AppEvent::FailureReported(error.to_string()));
                                }
                            }
                        }
                    }
                    AppCommand::Quit => return Ok(TuiExit::UserRequested),
                    AppCommand::Interrupt => {
                        let refresh = refresh_turn(
                            &mut client,
                            &mut conversation,
                            &mut active_turn,
                            &mut app,
                        );
                        if let ThreadRefresh::Applied { sequence } = refresh {
                            thread_subscription.confirm_sequence(sequence);
                        }
                        if let Some(turn_id) = active_turn.clone()
                            && !matches!(app.status(), Status::Error)
                        {
                            let refresh = interrupt_turn(
                                &mut client,
                                &mut conversation,
                                &turn_id,
                                &mut active_turn,
                                &mut app,
                            );
                            if let ThreadRefresh::Applied { sequence } = refresh {
                                thread_subscription.confirm_sequence(sequence);
                            }
                        } else if !matches!(app.status(), Status::Ready) {
                            app.update(AppEvent::InterruptFailed(
                                "the active turn is not available".into(),
                            ));
                        }
                    }
                    AppCommand::ReadClipboardImage => app.update(AppEvent::ClipboardImageRead(
                        host::clipboard::read_image().map(|image| image.png),
                    )),
                    AppCommand::SetSkillEnablement {
                        skill_id,
                        enablement,
                    } => match skills::set_enablement(&mut client, skill_id, enablement) {
                        Ok(view) => app.update(AppEvent::SkillsViewReplaced(view)),
                        Err(error) => app.update(AppEvent::FailureReported(error.to_string())),
                    },
                    AppCommand::SubmitTurn(prompt) => {
                        terminal.draw(|terminal_frame| frame::draw(terminal_frame, &app))?;
                        active_turn =
                            submit_prompt(&mut client, &mut conversation, prompt, &mut app);
                    }
                }
            }
            terminal.draw(|terminal_frame| frame::draw(terminal_frame, &app))?;
        }
    })();
    let pump_result = pump.shutdown();
    match (result, pump_result) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error.into()),
        (Ok(exit), Ok(())) => Ok(exit),
    }
}

fn refresh_server_event<T>(
    event: client::ClientEvent,
    client: &mut AppServerClient<T>,
    conversation: &mut ActiveConversation,
    active_turn: &mut Option<zeta_protocol::TurnId>,
    thread_subscription: &mut ThreadSubscription,
    app: &mut App,
) where
    T: JsonRpcTransport,
{
    let refresh_thread_snapshot = match event {
        client::ClientEvent::SkillsChanged if app.skills_view_is_active() => {
            match skills::load_selection(
                client,
                zeta_app_server_protocol::protocol::skills::SkillCatalogReloadDto::Cached,
            ) {
                Ok(view) => app.update(AppEvent::SkillsViewReplaced(view)),
                Err(error) => app.update(AppEvent::FailureReported(error.to_string())),
            }
            false
        }
        client::ClientEvent::Failed(error) => {
            app.update(AppEvent::FailureReported(error));
            false
        }
        client::ClientEvent::ThreadUpdated(update) => {
            thread_subscription.requires_snapshot(&update)
        }
        client::ClientEvent::SkillsChanged => false,
    };
    if refresh_thread_snapshot
        && let ThreadRefresh::Applied { sequence } =
            refresh_turn(client, conversation, active_turn, app)
    {
        thread_subscription.confirm_sequence(sequence);
    }
}

fn submit_prompt<T>(
    client: &mut AppServerClient<T>,
    conversation: &mut ActiveConversation,
    submission: ComposerSubmission,
    app: &mut App,
) -> Option<TurnId>
where
    T: JsonRpcTransport,
{
    match request_submit_prompt(client, thread_request_scope(conversation), submission) {
        Ok(start) => {
            conversation.set_thread_sequence(start.sequence);
            Some(start.turn_id)
        }
        Err(error) => {
            app.update(AppEvent::FailureReported(error.to_string()));
            None
        }
    }
}

fn refresh_turn<T>(
    client: &mut AppServerClient<T>,
    conversation: &mut ActiveConversation,
    active_turn: &mut Option<TurnId>,
    app: &mut App,
) -> ThreadRefresh
where
    T: JsonRpcTransport,
{
    match read_thread(client, conversation.thread_id()) {
        Ok(snapshot) => {
            if snapshot.session_id != *conversation.session_id()
                || snapshot.thread_id != *conversation.thread_id()
            {
                app.update(AppEvent::FailureReported(format!(
                    "thread/read returned snapshot for {}/{}; expected {}/{}",
                    snapshot.session_id,
                    snapshot.thread_id,
                    conversation.session_id(),
                    conversation.thread_id()
                )));
                return ThreadRefresh::Failed;
            }
            conversation.set_thread_sequence(snapshot.sequence);
            let sequence = snapshot.sequence;
            let active_turn_update = evaluate_active_turn(active_turn, &snapshot.turns);
            app.update(AppEvent::ThreadSnapshotReceived(snapshot));
            apply_active_turn_update(app, active_turn_update);
            ThreadRefresh::Applied { sequence }
        }
        Err(error) => {
            app.update(AppEvent::FailureReported(error.to_string()));
            ThreadRefresh::Failed
        }
    }
}

fn interrupt_turn<T>(
    client: &mut AppServerClient<T>,
    conversation: &mut ActiveConversation,
    turn_id: &TurnId,
    active_turn: &mut Option<TurnId>,
    app: &mut App,
) -> ThreadRefresh
where
    T: JsonRpcTransport,
{
    match request_interrupt_turn(client, thread_request_scope(conversation), turn_id) {
        Ok(result) => {
            conversation.set_thread_sequence(result.sequence);
            refresh_turn(client, conversation, active_turn, app)
        }
        Err(error) => {
            app.update(AppEvent::InterruptFailed(error.to_string()));
            ThreadRefresh::Failed
        }
    }
}

#[cfg(test)]
pub(crate) fn apply_active_turn_snapshot(
    app: &mut App,
    active_turn: &mut Option<TurnId>,
    turns: &[Turn],
) {
    apply_active_turn_update(app, evaluate_active_turn(active_turn, turns));
}

fn apply_active_turn_update(app: &mut App, update: ActiveTurnUpdate) {
    match update {
        ActiveTurnUpdate::ActivityChanged(activity) => {
            app.update(AppEvent::TurnActivityChanged(activity));
        }
        ActiveTurnUpdate::Completed => app.update(AppEvent::TurnCompleted),
        ActiveTurnUpdate::Failed(error) => app.update(AppEvent::FailureReported(error)),
        ActiveTurnUpdate::Interrupted => app.update(AppEvent::TurnInterrupted),
        ActiveTurnUpdate::Unchanged => {}
    }
}

fn thread_request_scope(conversation: &ActiveConversation) -> ThreadRequestScope {
    ThreadRequestScope::new(
        conversation.session_id(),
        conversation.thread_id(),
        conversation.thread_sequence(),
    )
}

fn sync_file_search_query(app: &App, file_search: &mut FileSearchManager) {
    if let Some(query) = app.mention_query() {
        file_search.update_query(query);
    } else {
        file_search.stop();
    }
}
