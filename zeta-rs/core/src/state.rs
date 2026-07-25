use crate::CoreError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TurnStatus {
    Created,
    Running,
    WaitingForApproval,
    WaitingForUserInput,
    WaitingForCapability,
    Cancelling,
    Completed,
    Failed,
    Interrupted,
}

impl TurnStatus {
    pub fn transition(self, next: Self) -> Result<Self, CoreError> {
        let allowed = matches!(
            (self, next),
            (Self::Created, Self::Running)
                | (
                    Self::Running,
                    Self::WaitingForApproval
                        | Self::WaitingForUserInput
                        | Self::WaitingForCapability
                        | Self::Completed
                        | Self::Failed
                        | Self::Cancelling
                )
                | (
                    Self::WaitingForApproval
                        | Self::WaitingForUserInput
                        | Self::WaitingForCapability,
                    Self::Running | Self::Cancelling
                )
                | (Self::Cancelling, Self::Interrupted)
        );
        if allowed {
            Ok(next)
        } else {
            Err(CoreError::InvalidTransition {
                from: format!("{self:?}"),
                to: format!("{next:?}"),
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ItemStatus {
    Created,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl ItemStatus {
    pub fn transition(self, next: Self) -> Result<Self, CoreError> {
        let allowed = matches!(
            (self, next),
            (Self::Created, Self::InProgress)
                | (
                    Self::InProgress,
                    Self::Completed | Self::Failed | Self::Cancelled
                )
        );
        if allowed {
            Ok(next)
        } else {
            Err(CoreError::InvalidTransition {
                from: format!("{self:?}"),
                to: format!("{next:?}"),
            })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolCallStatus {
    Proposed,
    AwaitingApproval,
    Running,
    Succeeded,
    Failed,
    Declined,
    Cancelled,
}

impl ToolCallStatus {
    pub fn transition(self, next: Self) -> Result<Self, CoreError> {
        let allowed = matches!(
            (self, next),
            (Self::Proposed, Self::AwaitingApproval | Self::Running)
                | (
                    Self::AwaitingApproval,
                    Self::Running | Self::Declined | Self::Cancelled
                )
                | (
                    Self::Running,
                    Self::Succeeded | Self::Failed | Self::Cancelled
                )
        );
        if allowed {
            Ok(next)
        } else {
            Err(CoreError::InvalidTransition {
                from: format!("{self:?}"),
                to: format!("{next:?}"),
            })
        }
    }
}
