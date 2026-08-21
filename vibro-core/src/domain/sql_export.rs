use std::marker::PhantomData;
use sal_core::dbg::Dbg;
use crate::Eval;

pub struct SqlExport<SqlBuilder, Ctx, Child> {
    /// Замыкание в котором формируются SQL запросы.
    builder: SqlBuilder,
    /// Предыдущий узел конвейера вычислений (например, угловой ресемплер или оконный фильтр).
    child: Child,
    _ctx: PhantomData<Ctx>, 
    /// Полное имя узла для отладки
    dbg: Dbg,
}
//
impl<F, Ctx, Child> SqlExport<F, Ctx, Child>
where
    F: Fn(&Ctx),
    Child: Eval<Ctx, Ctx> + Send + 'static {
    ///
    /// ### Returns `SqlExport` new instance
    /// - `parent` - Идентификатор родительской сущности (для отладки).
    /// - `builder` - Замыкание в котором формируются SQL запросы.
    /// - `child` - Дочерний (предыдущий) расчетный шаг
    pub fn new(parent: &Dbg, builder: F, child: Child) -> Self {
        let dbg = Dbg::new(parent, crate::me::<Self>());
        Self {
            builder,
            child,
            _ctx: PhantomData,
            dbg,
        }
    }
}
//
impl<F, Ctx, Child> Eval<Ctx, Ctx> for SqlExport<F, Ctx, Child>
where
    F: Fn(&Ctx),
    Child: Eval<Ctx, Ctx> + Send + 'static {
    //
    #[inline]
    fn eval(&self, ctx: Ctx) -> Ctx {
        let ctx = self.child.eval(ctx);
        // if ctx.err.is_some() {
        //     return ctx.pass_err(&self.dbg, "eval");
        // }
        (self.builder)(&ctx);
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
