use crate::index;
use std::fmt;
use std::fmt::Display;
use tantivy::directory::MmapDirectory;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, ReloadPolicy, IndexWriter, IndexReader};

const BULK_COUNT: usize = 100;

#[derive(Debug, Clone)]
pub struct TantivyOptions {
    index_dir: String,
}

impl Display for TantivyOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.index_dir,)
    }
}

#[derive(Clone)]
pub struct Tantivy {
    index: Index,
    reader: IndexReader,

    qp: QueryParser,

    options: TantivyOptions,
}

impl Display for Tantivy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.options)
    }
}

impl Tantivy {
    pub fn new(options: TantivyOptions) -> Self {

        let mut schema_builder = Schema::builder();

        schema_builder.add_text_field("id", STORED);
        schema_builder.add_text_field("type", TEXT);
        schema_builder.add_text_field("name", TEXT);
        schema_builder.add_text_field("desc", TEXT);

        let schema = schema_builder.build();
        let index = Index::open_or_create(MmapDirectory::open(&options.index_dir).unwrap(), schema.clone()).unwrap();

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into().unwrap();

        let name = schema.get_field("name").unwrap();
        let desc = schema.get_field("desc").unwrap();
        let qp = QueryParser::for_index(&index, vec![name, desc]);

        Self { options, index, reader, qp }
    }
}

impl Default for TantivyOptions {
    fn default() -> Self {
        Self {
            index_dir: String::from("./index/"),
        }
    }
}

impl index::Indexer for Tantivy {
    fn index<T: index::Index>(&self, idx: Box<T>) -> Result<(), index::IndexError> {
        let writer = self.index.writer(500);
        Ok(())
    }
    fn index_bulk<T: index::Index>(&self, curs: Vec<Box<T>>) -> Result<(), index::IndexError> {
        let mut writer = self.index.writer(50_000_000).unwrap();
        let mut i = 0;
        for idx in curs.into_iter() {
            let mut doc = Document::default();
            for (j, t) in idx.tuples().iter().enumerate() {
                if j == 0 {
                    let id_field = self.index.schema().get_field("id").unwrap();
                    doc.add_text(id_field, &t.2);
                }
                let field = self.index.schema().get_field(&t.1).unwrap();
                doc.add_text(field, &t.3);
            }
            writer.add_document(doc);
            i += 1;

            if i % BULK_COUNT == 0 {
                if let Err(e) = writer.commit() {
                    error!("index writer commit failed {}", e);
                }
            }
        }
        if let Err(e) = writer.commit() {
            error!("index writer commit failed {}", e);
            return Err(index::IndexError::ProcessingError);
        }
        Ok(())
    }

    fn query(&self, col: &str, qs: &str) -> Result<Vec<String>, index::IndexError> {
        let searcher = self.reader.searcher();

        let query = self.qp.parse_query(qs).unwrap();

        let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();
        let ids = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address).unwrap();
            println!("{}", self.index.schema().to_json(&retrieved_doc));
        }

        Ok(ids)
    }
    fn flush_all(&self, col: &str) -> Result<(), index::IndexError> {
        // Index::create_in_dir(&self.options.index_dir, self.index.schema()).unwrap();
        Ok(())
    }
}
