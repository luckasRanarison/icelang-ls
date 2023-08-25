use lazy_static::lazy_static;

#[rustfmt::skip]
pub const KEYWORDS: [&str; 14] = [
    "null",
    "true",
    "false",
    "set",
    "function",
    "lambda",
    "for",
    "in",
    "to",
    "while",
    "loop",
    "if",
    "else",
    "match",
];

pub struct BuiltinFn {
    pub name: String,
    pub args: Vec<String>,
    pub doc: String,
}

impl BuiltinFn {
    fn new(name: &str, args: Vec<&str>, doc: &str) -> Self {
        Self {
            name: name.to_owned(),
            args: args.iter().map(|s| s.to_string()).collect(),
            doc: doc.to_owned(),
        }
    }
}

lazy_static! {
    pub static ref BUILTIN_FUNCTION: Vec<BuiltinFn> = vec![
        BuiltinFn::new(
            "print",
            vec!["args"],
            "Print arguments to standard output. Example: ``` print('Hello World')```",
        ),
        BuiltinFn::new(
            "readline",
            vec![],
            "Read from standard input. Example: ```set input = readline()```"
        ),
        BuiltinFn::new(
            "import",
            vec!["value"],
            "Import value from a module, Example: ``` set module = import('module')```"
        ),
        BuiltinFn::new(
            "export",
            vec!["value"],
            "Returns a value from a script. Example: ```export(my_object)```"
        ),
        BuiltinFn::new(
            "type_of",
            vec!["value"],
            "Returns the type of the argument. Example: ```type_of('string') -- string```"
        ),
        BuiltinFn::new(
            "length",
            vec!["value"],
            "Returns the length of iterable types. Example: ```length([1, 2, 3]) -- 3```"
        ),
        BuiltinFn::new(
            "parse_number",
            vec!["number"],
            "Parse string to number. Example: ```parse_number('2') -- 2```"
        ),
        BuiltinFn::new(
            "sqrt",
            vec!["number"],
            "Returns the square root of a number"
        ),
        BuiltinFn::new(
            "floor",
            vec!["number"],
            "Returns the largest integer less than or equal to a number"
        ),
        BuiltinFn::new(
            "round",
            vec!["number"],
            "Rounds a number to the nearest integer"
        ),
        BuiltinFn::new(
            "ceil",
            vec!["number"],
            "Returns the smallest integer greater than or equal to a number"
        ),
        BuiltinFn::new(
            "pow",
            vec!["number", "exp"],
            "Raises the number to the power of exp"
        ),
    ];
}
