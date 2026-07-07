pub trait Eval<In, Out> {
    ///
    /// Perform an operation
    fn eval(&self, val: In) -> Out;
    ///
    /// Halt an operation
    fn exit(&self);
}
