use zeta_app_server_protocol::protocol::projects::ProjectDto;
use zeta_app_server_protocol::protocol::projects::ProjectRootDto;
use zeta_app_server_protocol::protocol::projects::ProjectStatusDto;
use zeta_app_server_protocol::protocol::projects::ProjectSummaryDto;
use zeta_projects::Project;
use zeta_projects::ProjectStatus;

pub(super) fn project(project: &Project) -> ProjectDto {
    ProjectDto {
        project_id: project.project_id.clone(),
        revision: project.revision,
        status: status(project.status),
        name: project.name.clone(),
        description: project.description.clone(),
        roots: project
            .roots
            .values()
            .map(|root| ProjectRootDto {
                environment_id: root.environment_id.clone(),
                dir_id: root.dir_id.clone(),
                path: root.path.clone(),
                name: root.name.clone(),
                purpose: root.purpose.clone(),
            })
            .collect(),
        session_ids: project.session_ids.iter().cloned().collect(),
        work_run_ids: project.work_run_ids.iter().cloned().collect(),
    }
}

pub(super) fn summary(project: &Project) -> ProjectSummaryDto {
    ProjectSummaryDto {
        project_id: project.project_id.clone(),
        revision: project.revision,
        status: status(project.status),
        name: project.name.clone(),
        root_count: project.roots.len() as u64,
        session_count: project.session_ids.len() as u64,
        work_run_count: project.work_run_ids.len() as u64,
    }
}

fn status(status: ProjectStatus) -> ProjectStatusDto {
    match status {
        ProjectStatus::Active => ProjectStatusDto::Active,
        ProjectStatus::Archived => ProjectStatusDto::Archived,
    }
}
