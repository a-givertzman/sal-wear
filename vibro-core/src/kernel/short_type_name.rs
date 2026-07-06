///
/// ### Извлекает короткое имя типа без пути модуля.
/// Например: "cma_server::task::FnRetain" -> "FnRetain"
pub fn short_type_name<T: ?Sized>() -> &'static str {
    let full = std::any::type_name::<T>();
    full.rsplit("::").next().unwrap_or(full)
}
pub use short_type_name as me;
