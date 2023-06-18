pub(crate) trait DetermineTaskState {
    fn determine_task_state(&self) -> bool;
}
