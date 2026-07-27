///
/// ### Создает новую ошибку на основе текущего контекста выполнения.
/// 
/// Добавь атрибут метода `#[named]` из крейта `function_name`
///
/// **Примеры использования:**
/// * `err!(self.dbg, "Текст")` — использует имя текущей функции и поле класса `self.dbg`.
/// * `err!(Self, "Текст")` — преобразует имя структуры `Self` в строку и использует её как ID класса.
#[macro_export]
macro_rules! err {
    // Ветка 1: Если передали ключевое слово Self
    (Self, $($arg:tt)+) => {
        $crate::Error::new(
            $crate::short_type_name::<Self>(), 
            function_name!()
        ).err(format!($($arg)+))
    };
    // Ветка 2: Если передали локальную переменную (например, id или self.dbg)
    ($ctx:expr, $($arg:tt)+) => {
        $crate::Error::new($ctx.to_string(), function_name!()).err(format!($($arg)+))
    };
}
///
/// ### Пробрасывает ошибку выше по стеку, оборачивая её в текущий контекст трассировки.
///
/// Добавь атрибут метода `#[named]` из крейта `function_name`
/// 
/// **Примеры использования:**
/// * `err_pass!(self.dbg, err)` — передает ошибку дальше с именем класса `self.dbg` и именем текущей функции.
/// * `err_pass!(self.dbg, err, "Описание")` — передает ошибку с дополнительным описанием и `self.dbg`.
/// * `err_pass!(Self, err)` — передает ошибку дальше, используя имя структуры `Self` как ID класса.
/// * `err_pass!(Self, err, "Описание")` — передает ошибку с описанием и именем `Self``.
#[macro_export]
macro_rules! err_pass {
    // Ветка 1: Чистый проброс для Self
    (Self, $err:expr) => {
        $crate::Error::new(
            $crate::me::<Self>(), 
            function_name!()
        ).pass($err.to_string())
    };
    // Ветка 2: Проброс с описанием для Self
    (Self, $err:expr, $($arg:tt)+) => {
        $crate::Error::new(
            $crate::short_type_name::<Self>(), 
            function_name!()
        ).pass_with(format!($($arg)+), $err.to_string())
    };
    // Ветка 3: Чистый проброс для переменной
    ($ctx:expr, $err:expr) => {
        $crate::Error::new($ctx.to_string(), function_name!()).pass($err.to_string())
    };
    // Ветка 4: Проброс с описанием для переменной
    ($ctx:expr, $err:expr, $($arg:tt)+) => {
        $crate::Error::new($ctx.to_string(), function_name!()).pass_with(format!($($arg)+), $err.to_string())
    };
}