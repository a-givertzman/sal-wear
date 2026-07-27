use std::marker::PhantomData;
use sal_core::dbg::Dbg;
use crate::{Eval, Sender};

pub struct SqlExport<E, F, Ctx, Child> {
    /// Канал передачи SQL запросов в сервис реализующий отправку в БД
    api_link: Sender<E>,
    /// Замыкание в котором формируются SQL запросы.
    builder: F,
    /// Предыдущий узел конвейера вычислений (например, угловой ресемплер или оконный фильтр).
    child: Child,
    _ctx: PhantomData<Ctx>, 
    /// Полное имя узла для отладки
    dbg: Dbg,
}
//
impl<E, F, Ctx, Child> SqlExport<E, F, Ctx, Child>
where
    F: Fn(&Ctx) -> Vec<E>,
    Child: Eval<Ctx, Ctx> + Send + 'static {
    ///
    /// ### Returns `SqlExport` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `builder` - Замыкание в котором формируются SQL запросы.
    /// - `child` - Дочерний (предыдущий) расчетный шаг
    pub fn new(parent: &Dbg, api_link: Sender<E>, builder: F, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            api_link,
            builder,
            child,
            _ctx: PhantomData,
            dbg,
        }
    }
}
//
impl<E, F, Ctx, Child> Eval<Ctx, Ctx> for SqlExport<E, F, Ctx, Child>
where
    F: Fn(&Ctx) -> Vec<E>,
    Child: Eval<Ctx, Ctx> + Send + 'static {
    //
    #[inline]
    fn eval(&self, ctx: Ctx) -> Ctx {
        let ctx = self.child.eval(ctx);
        // if ctx.err.is_some() {
        //     return ctx.pass_err(&self.dbg, "eval");
        // }
        let sqls = (self.builder)(&ctx);
        for sql in sqls {
            if self.api_link.send(sql).is_err() {
                log::error!("{}.eval | Can't send sql, Api service disconnected", self.dbg);
                break;
            }
        }
        ctx
    }
    //
    fn exit(&self) {
        self.child.exit();
    }
}
/// Подготавливает сырую строку для безопасной вставки в SQL-запрос.
/// - Удаляет пробелы по краям
/// - Вырезает нулевые байты (\0)
/// - Экранирует одинарные кавычки
pub fn escape(input: &str) -> String {
    let trimmed = input.trim();
    // +8 байт — запас под несколько кавычек
    let mut result = String::with_capacity(trimmed.len() + 8);
    for c in trimmed.chars() {
        match c {
            '\0' => continue,
            '\'' => result.push_str("''"),
            _ => result.push(c),
        }
    }
    result
}
