pub(crate) trait EventTranslatorOneArg<T: Default + Copy, A> {
    fn translate_to(&self, event: &mut T, arg: A);
}

pub(crate) trait EventTranslatorTwoArg<T: Default + Copy, A, B> {
    fn translate_to(&self, event: &mut T, arg0: A, arg1: B);
}

pub(crate) trait EventTranslatorThreeArg<T: Default + Copy, A, B, C> {
    fn translate_to(&self, event: &mut T, arg0: A, arg1: B, arg2: C);
}

pub(crate) trait EventTranslatorFourArg<T: Default + Copy, A, B, C, D> {
    fn translate_to(&self, event: &mut T, arg0: A, arg1: B, arg2: C, arg3: D);
}

pub(crate) trait EventTranslatorFiveArg<T: Default + Copy, A, B, C, D, E> {
    fn translate_to(&self, event: &mut T, arg0: A, arg1: B, arg2: C, arg3: D, arg4: E);
}
