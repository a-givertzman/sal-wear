#
# Rust Сборщик (Bundler) с поддержкой исключения папок и очистки тестов
#
# Предназначен для объединения многофайловых Rust-проектов в один монолитный файл.
# Корректно обрабатывает вложенные модули, изолирует строковые литералы при очистке
# и позволяет исключать тестовые блоки и C-биндинги без разрушения синтаксиса.
#
# Варианты запуска:
# 1. Стандартный запуск (ищет src/main.rs в текущей папке):
#    python bundler.py
# 2. Указание конкретного файла:
#    python bundler.py src/bin/tools.rs
# 3. Запуск от определенной начальной папки с пропуском директорий:
#    python bundler.py src/main.rs --base-dir /path/to/project --exclude-dirs src/temp_tests
# 4. Полная очистка от тестов и внешних Си-функций:
#    python bundler.py --no-tests --no-c
#
# Пример:
# python3 ./bundler.py src/services/task/mod.rs --no-tests --exclude-dirs \
#    src/services/task/functions/application \
#    src/services/task/functions/common \
#    src/services/task/functions/comp \
#    src/services/task/functions/conversion \
#    src/services/task/functions/edge_detection \
#    src/services/task/functions/export \
#    src/services/task/functions/filter \
#    src/services/task/functions/import \
#    src/services/task/functions/io \
#    src/services/task/functions/ops \
#    src/services/task/functions/plot \
#    src/services/task/functions/sql \
#    src/services/task/functions/timers \
#    src/services/task/task_test_producer.rs \
#    src/services/task/task_test_receiver.rs
#
import re
import argparse
from pathlib import Path
class RustCodeProcessor:
    """
    Лексический анализатор для безопасной модификации исходного кода Rust.
    Использует конечный автомат для отслеживания контекста (строка, символ, блок кода),
    чтобы регулярные выражения не ломали синтаксис внутри текстовых литералов.
    """
    def __init__(self, code):
        self.code = code
    def remove_blocks(self, pattern_str):
        """
        Находит объявление блока (например, #[test]) и удаляет всё его содержимое
        вплоть до закрывающей фигурной скобки, вычисляемой через глубину вложенности.
        """
        pattern = re.compile(pattern_str, re.MULTILINE)
        while True:
            match = pattern.search(self.code)
            if not match:
                break
            start = match.start()
            end = self._find_item_end(match.end())
            self.code = self.code[:start] + self.code[end:]
        return self
    def _find_item_end(self, start_idx):
        """
        Внутренний механизм обхода AST. Считает скобки `{}`, 
        игнорируя их внутри кавычек и экранированных последовательностей.
        """
        depth = 0
        in_block = False
        in_string = False
        in_char = False
        escape = False
        i = start_idx
        while i < len(self.code):
            char = self.code[i]
            if escape:
                escape = False
                i += 1
                continue
            if char == '\\':
                escape = True
                i += 1
                continue
            if char == '"' and not in_char:
                in_string = not in_string
            elif char == "'" and not in_string:
                in_char = not in_char
            elif not in_string and not in_char:
                if char == '{':
                    depth += 1
                    in_block = True
                elif char == '}':
                    depth -= 1
                    if in_block and depth == 0:
                        return i + 1
                elif char == ';' and not in_block:
                    return i + 1
            i += 1
        return len(self.code)
    def remove_comments(self):
        """Посимвольно удаляет `//` и `/* */`, защищая пути и URL-адреса внутри строковых констант."""
        result = []
        in_string = False
        in_char = False
        escape = False
        i = 0
        while i < len(self.code):
            char = self.code[i]
            if escape:
                escape = False
                result.append(char)
                i += 1
                continue
            if char == '\\':
                escape = True
                result.append(char)
                i += 1
                continue
            if char == '"' and not in_char:
                in_string = not in_string
            elif char == "'" and not in_string:
                in_char = not in_char
            if not in_string and not in_char:
                if char == '/' and i + 1 < len(self.code):
                    next_char = self.code[i + 1]
                    if next_char == '/':
                        while i < len(self.code) and self.code[i] != '\n':
                            i += 1
                        continue
                    elif next_char == '*':
                        i += 2
                        while i + 1 < len(self.code) and not (self.code[i] == '*' and self.code[i+1] == '/'):
                            i += 1
                        i += 2
                        continue
            result.append(char)
            i += 1
        self.code = "".join(result)
        return self
    def clean_empty_lines(self):
        """Удаляет концевые пробелы и пустые строки для чистоты итогового файла."""
        lines = [line.rstrip() for line in self.code.splitlines() if line.strip()]
        self.code = "\n".join(lines) + "\n"
        return self
    def get_code(self):
        return self.code
class RustBundler:
    """
    Управляет рекурсивным обходом дерева зависимостей модулей.
    Инкапсулирует логику определения путей (Pathlib) для жесткой изоляции исключений.
    """
    def __init__(self, entry_file, custom_base_dir, exclude_dirs):
        self.entry_path = Path(entry_file).resolve()
        if custom_base_dir:
            self.base_dir = Path(custom_base_dir).resolve()
            if self.base_dir.is_file():
                self.base_dir = self.base_dir.parent
        else:
            src_dir = self._find_src_parent(self.entry_path)
            self.base_dir = src_dir if src_dir else self.entry_path.parent
        self.exclude_dirs = [(self.base_dir / d).resolve() for d in exclude_dirs]
    def _find_src_parent(self, path):
        """Ищет корневую папку проекта (src) для корректной работы относительных путей исключения."""
        current = path
        while current != current.parent:
            if current.name == 'src':
                return current.parent
            current = current.parent
        return None
    def is_excluded(self, path):
        """Проверяет принадлежность текущего файла или директории к черному списку."""
        abs_path = path.resolve()
        for ex in self.exclude_dirs:
            if abs_path.is_relative_to(ex) or abs_path == ex:
                return True
        return False
    def bundle(self, file_path):
        """Основной метод обхода: подменяет объявления модулей на их физическое содержимое."""
        path = Path(file_path).resolve()
        if self.is_excluded(path):
            print(f"⏭️  Пропущен: {path.name} (по правилам исключения)")
            return f"// Skipped: {path.name} (excluded)\n"
        if not path.exists():
            print(f"❌ ОШИБКА: Файл не найден - {path}")
            return f'compile_error!("Bundler Error: File not found - {path.name}");\n'
        with open(path, 'r', encoding='utf-8') as f:
            lines = f.readlines()
        result = []
        file_stem = path.stem
        # Логика Rust: подмодули файлов mod.rs лежат рядом, а подмодули file.rs — в папке file/
        if file_stem in ("mod", "main", "lib"):
            search_dir = path.parent
        else:
            search_dir = path.parent / file_stem
        for line in lines:
            match = re.match(r'^(?P<prefix>.*?\bmod\s+)(?P<name>\w+)\s*;', line)
            if match:
                mod_name = match.group('name')
                prefix = match.group('prefix')
                mod_path_file = search_dir / f"{mod_name}.rs"
                mod_path_dir = search_dir / mod_name / "mod.rs"
                target_path = None
                if mod_path_file.exists():
                    target_path = mod_path_file
                elif mod_path_dir.exists():
                    target_path = mod_path_dir
                if target_path:
                    if self.is_excluded(target_path):
                        print(f"⏭️  Пропущен модуль: {mod_name}")
                        result.append(f"// {prefix}{mod_name}; (Excluded)\n")
                    else:
                        result.append(f"{prefix}{mod_name} {{\n")
                        result.append(self.bundle(target_path))
                        result.append(f"}}\n")
                else:
                    print(f"⚠️ ВНИМАНИЕ: Модуль '{mod_name}' объявлен в {path.name}, но файл не найден.")
                    result.append(line)
            else:
                result.append(line)
        return "".join(result)
def main():
    parser = argparse.ArgumentParser(description="Rust Bundler")
    parser.add_argument("input", nargs='?', default="src/main.rs", help="Path to entry file (default: src/main.rs)")
    parser.add_argument("-o", "--output", default="bundle.rs", help="Output file")
    parser.add_argument("--base-dir", default=None, help="Force root directory for relative exclusion paths")
    parser.add_argument("--no-tests", action="store_true", help="Exclude #[test] blocks")
    parser.add_argument("--no-c", action="store_true", help="Exclude extern 'C' blocks")
    parser.add_argument("--exclude-dirs", nargs='+', default=[], help="List of relative directories to skip")
    args = parser.parse_args()
    print(f"🔄 Processing entry point: {args.input}...")
    bundler = RustBundler(args.input, args.base_dir, args.exclude_dirs)
    print(f"📁 Auto-detected project root: {bundler.base_dir}")
    full_code = bundler.bundle(bundler.entry_path)
    print("🧹 Cleaning code structures...")
    processor = RustCodeProcessor(full_code).remove_comments()
    if args.no_tests:
        print("🚫 Stripping test components...")
        processor.remove_blocks(r'#\[test\]').remove_blocks(r'#\[cfg\(test\)\]')
    if args.no_c:
        print("🚫 Stripping C-bindings...")
        processor.remove_blocks(r'extern\s+"C"')
    final_code = processor.clean_empty_lines().get_code()
    with open(args.output, 'w', encoding='utf-8') as f:
        f.write(final_code)
    print(f"✅ Success! Saved to {args.output}")
if __name__ == "__main__":
    main()
