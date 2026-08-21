///
/// ### Извлекает короткое имя типа без пути модуля.
/// Например: "cma_server::task::FnRetain" -> "FnRetain"
pub fn short_type_name<T: ?Sized>() -> String {
    pretty_type_name::pretty_type_name::<T>()
}
pub use short_type_name as me;
