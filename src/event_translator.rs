pub(crate) trait EventTranslatorOneArg<T: Default + Copy, A> {
    fn translate_to(&self, event: &mut T, arg: A);
}
