//! Iterator for format nodes

use format::*;

#[derive(PartialEq)]
enum FormatIterCmd<'a> {
    Iteration(usize, &'a FormatNode),
    Index(usize, &'a[FormatNode]),
    Walk(&'a FormatNode),
}

pub struct FormatEvalIter<'a> {
    stack: Vec<FormatIterCmd<'a>>,
}

impl<'a> Iterator for FormatEvalIter<'a> {
    type Item = &'a FormatNode;
    fn next(&mut self) -> Option<&'a FormatNode> {
        use self::FormatIterCmd::*;
        use format::FormatNode::*;
        loop {
            let last = match self.stack.pop() {
                Some(l) => l,
                None => return None,
            };
            match last {
                Index(pos, slice) => {
                    if pos < slice.len() {
                        self.stack.push(Index(pos + 1, slice));
                        self.stack.push(Walk(&slice[pos]));
                    }
                },
                Iteration(it, node) => {
                    if it > 0 {
                        self.stack.push(Iteration(it - 1, node));
                        self.stack.push(Walk(node));
                    }
                },
                Walk(node) => {
                    match node {
                        &Group(ref v) => {
                            self.stack.push(Index(0, &v));
                        },
                        &Repeat(r, ref node) => {
                            self.stack.push(Iteration(r, &node));
                        },
                        x => return Some(x),
                    }
                },
            }
        }
    }
}

impl<'a> IntoIterator for &'a FormatNode {
    type Item = &'a FormatNode;
    type IntoIter = FormatEvalIter<'a>;

    fn into_iter(self) -> FormatEvalIter<'a> {
        FormatEvalIter {
            stack: vec![FormatIterCmd::Walk(&self)],
        }
    }
}
