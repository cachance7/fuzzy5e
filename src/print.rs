use bson::{Bson, Document};
use serde_json::Value;
use tuikit::prelude::*;

pub trait Print {
    fn print(&self, doc: Document);
}

pub struct ResultPrinter {}

impl Print for ResultPrinter {
    fn print(&self, document: Document) {
        let doc: Value = Bson::from(document).clone().into();
        println!("{}", doc);
    }
}

pub struct FieldResultPrinter {
    field: String,
}

impl Print for FieldResultPrinter {
    fn print(&self, document: Document) {
        println!("{}", document.get_str(&self.field).unwrap());
    }
}

// pub struct TermPrinter {
//     term: Term,
// }

pub enum Printer {
    FieldResult(FieldResultPrinter),
    DocResult(ResultPrinter),
}

impl Print for Printer {
    fn print(&self, doc: Document) {
        match self {
            Printer::DocResult(p) => p.print(doc),
            Printer::FieldResult(p) => p.print(doc),
        }
    }
}

impl Printer {
    pub fn from_field_option(field: Option<String>) -> Printer {
        match field {
            Some(name) => Printer::FieldResult(FieldResultPrinter { field: name }),
            None => Printer::DocResult(ResultPrinter {}),
        }
    }
}
