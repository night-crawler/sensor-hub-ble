pub trait Command: Copy {
    fn address(self) -> u8;
}
