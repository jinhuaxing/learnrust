use std::{fmt::Display, num::ParseIntError};

type Tree = Box<Node>;

#[derive(Debug)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::Add => write!(f, "+"),
            Op::Sub => write!(f, "-"),
            Op::Mul => write!(f, "*"),
            Op::Div => write!(f, "/"),
        }
    }
}

impl Op {
    fn from_token(token: &Token) -> Op {
        match token {
            Token::ADD => Op::Add,
            Token::SUB => Op::Sub,
            Token::MUL => Op::Mul,
            Token::DIV => Op::Div,
            _ => panic!("Not Op"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Token {
    LPAR,
    RPAR,
    ADD,
    SUB,
    MUL,
    DIV,
    NUMBER(String),
    END,
}

impl Token {
    fn single_char_token(ch: char) -> Self {
        match ch {
            '+' => Self::ADD,
            '-' => Self::SUB,
            '*' => Self::MUL,
            '/' => Self::DIV,
            '(' => Self::LPAR,
            ')' => Self::RPAR,
            _ => {
                panic!("Unknown single char token")
            }
        }
    }
}

#[derive(Debug)]
enum Node {
    Par(Box<Node>),
    Expr { op: Op, left: Tree, right: Tree },
    Number(String),
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let input = input.as_bytes();
    let mut result = Vec::new();
    let mut index = 0;
    while index < input.len() {
        let current_char = input[index] as char;
        match current_char {
            '(' | ')' | '+' | '-' | '*' | '/' => {
                result.push(Token::single_char_token(current_char));
            }

            '0'..='9' => {
                let mut num_index = index + 1;
                while num_index < input.len() {
                    let ch = input[num_index] as char;
                    if ch >= '0' && ch <= '9' {
                        num_index += 1;
                    } else {
                        break;
                    }
                }
                let n = Token::NUMBER(String::from_utf8(input[index..num_index].to_vec()).unwrap());
                result.push(n);
                index = num_index - 1;
            }
            ' ' | '\t' => {
                // Skip
            }

            _ => {
                return Err("Unknown char found".to_string());
            }
        };
        index += 1;
    }

    result.push(Token::END);
    Ok(result)
}

/*
 Expr   := Term Expr1
 Expr1  := '+' Term Expr1 | Empty
 Term   := Factor Term1
 Term1  := '*' Factor Term1 | Empty
 Factor := '(' Expr ')' | Number
*/
struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, current: 0 }
    }

    fn expr(&mut self) -> Tree {
        let term = self.term();
        self.expr1(term)
    }

    fn expr1(&mut self, left: Tree) -> Tree {
        let token = &self.tokens[self.current];
        if *token == Token::ADD || *token == Token::SUB {
            self.current += 1;
            let term = self.term();
            let t = Box::new(Node::Expr {
                op: Op::from_token(token),
                left: left,
                right: term,
            });
            self.expr1(t)
        } else {
            left
        }
    }

    fn term(&mut self) -> Tree {
        let factor = self.factor();
        self.term1(factor)
    }

    fn term1(&mut self, left: Tree) -> Tree {
        let token = &self.tokens[self.current];
        if *token == Token::MUL || *token == Token::DIV {
            self.current += 1;
            let factor = self.factor();
            let t = Box::new(Node::Expr {
                op: Op::from_token(token),
                left: left,
                right: factor,
            });
            self.term1(t)
        } else {
            left
        }
    }

    fn factor(&mut self) -> Tree {
        match self.tokens[self.current] {
            Token::LPAR => {
                self.current += 1;
                let expr = self.expr();
                if self.tokens[self.current] != Token::RPAR {
                    panic!("RPAR expected");
                } else {
                    self.current += 1;
                    Box::new(Node::Par(expr))
                }
            }
            Token::NUMBER(ref num) => {
                self.current += 1;
                Box::new(Node::Number(num.clone()))
            }
            _ => {
                panic!("Factor expected")
            }
        }
    }
}

fn eval_tree(tree: &Tree) -> Result<i64, ParseIntError> {
    match **tree {
        Node::Expr {
            ref op,
            ref left,
            ref right,
        } => {
            let vl = eval_tree(left)?;
            let vr = eval_tree(right)?;
            let v = match op {
                Op::Add => vl + vr,
                Op::Sub => vl - vr,
                Op::Mul => vl * vr,
                Op::Div => vl / vr,
            };
            Ok(v)
        }

        Node::Par(ref expr) => eval_tree(expr),

        Node::Number(ref num) => num.parse::<i64>(),
    }
}

fn print_tree(tree: &Tree) {
    match **tree {
        Node::Expr {
            ref op,
            ref left,
            ref right,
        } => {
            print_tree(left);
            print!("{}", op);
            print_tree(right);
        }

        Node::Par(ref expr) => {
            print!("(");
            print_tree(expr);
            print!(")");
        }

        Node::Number(ref num) => {
            print!("{}", num);
        }
    }
}

fn main() {
    let input = "(1+2-3)+4*5 -9 /8*7+((6+7))";
    let tokens = tokenize(input).unwrap();
    let mut p = Parser::new(&tokens[..]);
    let e = p.expr();
    println!("{:?}", p.tokens[p.current]);
    print_tree(&e);
    println!("");
    println!("{}", eval_tree(&e).unwrap());
}
