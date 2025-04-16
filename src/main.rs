use std::{
    collections::HashMap,
    fmt::Display,
    fs,
    io::{BufReader, Read},
};

use iter_tools::Itertools;

#[derive(Debug, Clone)]
enum Program {
    Lambda {
        arg: String,
        body: Box<Program>,
    },
    Application {
        fun: Box<Program>,
        arg: Box<Program>,
    },
    Variable {
        name: String,
    },
}

impl Default for Program {
    fn default() -> Self {
        Program::Variable {
            name: "ERROR".to_string(),
        }
    }
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Program::Lambda { arg, body } => {
                write!(f, "; {arg} {body}")?;
            }
            Program::Application { fun, arg } => {
                write!(f, ". {fun} {arg}")?;
            }
            Program::Variable { name } => {
                name.fmt(f)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
enum Token {
    Lambda,
    Application,
    Operator(u32),
    OperatorInited(String),
    Program(Program),
}
impl Token {
    fn label(name: String) -> Token {
        Token::Program(Program::Variable { name })
    }
}

#[derive(Debug)]
struct TokenUnit {
    left: u32,
    token: Token,
}

fn main() {
    let filename = std::env::args().nth(1).unwrap();
    let file = BufReader::new(fs::File::open(filename).unwrap());
    let bytes = file.bytes();
    let mut token_stack: Vec<TokenUnit> = vec![];
    let mut operators: HashMap<String, Vec<u32>> = HashMap::new();
    let mut comment_level = 0u32;
    for token in bytes
        .map(Result::unwrap)
        .group_by(|&c| !c.is_ascii_whitespace())
        .into_iter()
        .filter_map(|v| if v.0 { Some(v.1) } else { None })
        .map(|s| String::from_utf8(s.collect()).unwrap())
    {
        if token == "]]" {
            assert!(comment_level > 0);
            comment_level -= 1;
            continue;
        }
        if token == "[[" {
            comment_level += 1;
        }
        if comment_level > 0 {
            continue;
        }
        match token.as_str() {
            ";" => token_stack.push(TokenUnit {
                left: 2,
                token: Token::Lambda,
            }),
            "." => token_stack.push(TokenUnit {
                left: 2,
                token: Token::Application,
            }),
            _ => {
                if let Some(num) = token.strip_prefix("#") {
                    let num: u32 = if num.len() > 0 {
                        num.parse().unwrap()
                    } else {
                        0
                    };
                    token_stack.push(TokenUnit {
                        left: 2,
                        token: Token::Operator(num),
                    });
                } else {
                    if let Some(op) = operators.get(&token) {
                        let num = *op.first().unwrap();
                        for _ in 0..num {
                            token_stack.push(TokenUnit {
                                left: 2,
                                token: Token::Application,
                            })
                        }
                    }
                    token_stack.push(TokenUnit {
                        left: token_stack.last().unwrap().left - 1,
                        token: Token::label(token),
                    });
                    while token_stack.last().unwrap().left == 0 {
                        let mut args = vec![];
                        loop {
                            match token_stack.pop().unwrap().token {
                                Token::Program(Program::Variable { name }) => {
                                    args.push(Token::label(name))
                                }
                                Token::Program(program) => args.push(Token::Program(program)),
                                Token::Operator(num) => {
                                    if let [body, Token::Program(Program::Variable { name })] =
                                        TryInto::<[_; 2]>::try_into(std::mem::take(&mut args))
                                            .unwrap()
                                    {
                                        operators.entry(name.clone()).or_default().push(num);
                                        token_stack.push(TokenUnit {
                                            left: 2,
                                            token: Token::OperatorInited(name),
                                        });
                                        token_stack.push(TokenUnit {
                                            left: 1,
                                            token: body,
                                        });
                                        break;
                                    } else {
                                        panic!();
                                    }
                                }
                                Token::Lambda => {
                                    if let [Token::Program(body), Token::Program(Program::Variable { name: arg })] =
                                        TryInto::<[_; 2]>::try_into(std::mem::take(&mut args))
                                            .unwrap()
                                    {
                                        token_stack.push(TokenUnit {
                                            left: token_stack
                                                .last()
                                                .map_or(1000001, |v| v.left - 1),
                                            token: Token::Program(Program::Lambda {
                                                arg,
                                                body: Box::new(body),
                                            }),
                                        });
                                        break;
                                    } else {
                                        dump_stack(token_stack);
                                        panic!()
                                    }
                                }
                                Token::Application => {
                                    if let [Token::Program(arg), Token::Program(fun)] =
                                        TryInto::<[_; 2]>::try_into(std::mem::take(&mut args))
                                            .unwrap()
                                    {
                                        token_stack.push(TokenUnit {
                                            left: token_stack
                                                .last()
                                                .map_or(1000002, |v| v.left - 1),
                                            token: Token::Program(Program::Application {
                                                fun: Box::new(fun),
                                                arg: Box::new(arg),
                                            }),
                                        });
                                        break;
                                    } else {
                                        panic!()
                                    }
                                }
                                Token::OperatorInited(name) => {
                                    if let [Token::Program(program), Token::Program(op)] =
                                        TryInto::<[_; 2]>::try_into(std::mem::take(&mut args))
                                            .unwrap()
                                    {
                                        let op_stack = operators.get_mut(&name).unwrap();
                                        op_stack.pop();
                                        if op_stack.is_empty() {
                                            operators.remove(&name);
                                        }
                                        token_stack.push(TokenUnit {
                                            left: token_stack
                                                .last()
                                                .map_or(1000003, |v| v.left - 1),
                                            token: Token::Program(Program::Application {
                                                fun: Box::new(Program::Lambda {
                                                    arg: name,
                                                    body: Box::new(program),
                                                }),
                                                arg: Box::new(op),
                                            }),
                                        });
                                        break;
                                    } else {
                                        panic!()
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // dbg!(&token_stack);
    // if let Token::Program(program) = &token_stack[0].token {
    //     println!("{}", program);
    // }
    if comment_level > 0 {
        println!("WARNING: unclosed comment");
    }
    fn dump_stack(token_stack: Vec<TokenUnit>) {
        for token in token_stack {
            match token.token {
                Token::Program(p) => println!("\n{} {p}", token.left),
                t => println!("\n{} {t:?}", token.left),
            }
        }
    }
    if token_stack.len() != 1 {
        println!("Unbalanced token stack:");

        dump_stack(token_stack);

        panic!()
    }

    let Token::Program(mut program) = token_stack.pop().unwrap().token else {
        panic!()
    };

    fn b_reduce(program: &mut Program) -> bool {
        b_reduce_(program, &mut HashMap::new())
    }

    fn b_reduce_(program: &mut Program, arg_set: &mut HashMap<String, u32>) -> bool {
        match program {
            Program::Lambda { body, arg } => {
                *arg_set.entry(arg.clone()).or_default() += 1;
                let res = b_reduce_(body, arg_set);
                *arg_set.get_mut(arg).expect("just inserted") -= 1;
                res
            }
            Program::Application { fun, arg } => {
                if let Program::Lambda { arg: name, body } = &mut **fun {
                    let arg = std::mem::take(arg);
                    apply(body, name, ArgVal::Owned(arg), &*arg_set);
                    let body = std::mem::take(body);
                    *program = *body;
                    return true;
                }
                b_reduce_(fun, arg_set) || b_reduce_(arg, arg_set)
            }
            _ => false,
        }
    }

    fn n_reduce(program: &mut Program) -> bool {
        match program {
            Program::Lambda { arg, body } => {
                if let Program::Application { fun, arg: arg2 } = &mut **body {
                    if let Program::Variable { name } = &**arg2 {
                        if name == arg && !seek(fun, name) {
                            let fun = std::mem::take(fun);
                            *program = *fun;
                            return true;
                        }
                    }
                }
                n_reduce(body)
            }
            Program::Application { fun, arg } => n_reduce(fun) || n_reduce(arg),
            _ => false,
        }
    }

    enum ArgVal<'a> {
        Owned(Box<Program>),
        Borrowed(&'a Program),
    }

    impl ArgVal<'_> {
        fn into_owned(self) -> Program {
            match self {
                ArgVal::Owned(program) => *program,
                ArgVal::Borrowed(program) => program.clone(),
            }
        }
        fn borrow(&self) -> &Program {
            match self {
                ArgVal::Owned(program) => program,
                ArgVal::Borrowed(program) => program,
            }
        }
    }

    fn apply<'a>(
        program: &'a mut Program,
        name: &str,
        arg_val: ArgVal<'a>,
        arg_set: &HashMap<String, u32>,
    ) -> ArgVal<'a> {
        match program {
            Program::Lambda { arg, body } => {
                if name != arg {
                    if arg_set.get(arg).copied().unwrap_or(0) != 0 && seek(arg_val.borrow(), arg) {
                        rename(body, &*arg);
                        arg.push('\'');
                    }
                    apply(body, name, arg_val, arg_set)
                } else {
                    arg_val
                }
            }
            Program::Application { fun, arg } => {
                let arg_val = apply(fun, name, arg_val, arg_set);
                apply(arg, name, arg_val, arg_set)
            }
            Program::Variable { name: name1 } => {
                if name1 == name {
                    *program = arg_val.into_owned();
                    ArgVal::Borrowed(program)
                } else {
                    arg_val
                }
            }
        }
    }

    fn seek(program: &Program, name: &str) -> bool {
        match program {
            Program::Lambda { arg, body } => name != arg && seek(body, name),
            Program::Application { fun, arg } => seek(fun, name) || seek(arg, name),
            Program::Variable { name: name1 } => name1 == name,
        }
    }

    fn rename(program: &mut Program, name: &str) {
        match program {
            Program::Lambda { arg, body } => {
                if arg.len() == name.len() + 1 && arg.starts_with(name) && arg.ends_with('\'') {
                    dbg!();
                    rename(body, arg);
                    arg.push('\'');
                }
                if name != arg {
                    rename(body, name);
                }
            }
            Program::Application { fun, arg } => {
                rename(fun, name);
                rename(arg, name);
            }
            Program::Variable { name: name1 } => {
                if name1 == name {
                    name1.push('\'');
                }
            }
        }
    }

    // println!("{}", program);
    while b_reduce(&mut program) || n_reduce(&mut program) {
        // println!("{}", program);
    }
    println!("{}", program);
}
