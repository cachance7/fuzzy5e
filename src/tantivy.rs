use crate::index;
use hex::ToHex;
use std::fmt;
use std::fmt::Display;
use tantivy::directory::MmapDirectory;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::tokenizer::*;
use tantivy::{Index, IndexReader, ReloadPolicy};

const BULK_COUNT: usize = 100;
const RESULT_LIMIT: usize = 30;

#[derive(Debug, Clone)]
pub struct TantivyOptions {
    pub index_dir: String,
    pub rebuild: bool,
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

        let en_stem_plus = TextAnalyzer::from(NgramTokenizer::new(2, 10, true))
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .filter(Stemmer::new(Language::English));

        let text_options = TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("en_stem_plus")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        );

        schema_builder.add_text_field("id", STORED);
        schema_builder.add_text_field("type", STORED);
        schema_builder.add_text_field("mtype", STORED);
        schema_builder.add_text_field("name_ng", text_options.clone());
        schema_builder.add_text_field("name", TEXT);
        schema_builder.add_text_field("desc", TEXT);
        schema_builder.add_text_field("doc", STORED);

        let schema = schema_builder.build();
        let index = if options.rebuild {
            Index::create_in_dir(&options.index_dir, schema.clone()).unwrap()
        } else {
            Index::open_or_create(
                MmapDirectory::open(&options.index_dir).unwrap(),
                schema.clone(),
            )
            .unwrap()
        };

        index.tokenizers().register("en_stem_plus", en_stem_plus);

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()
            .unwrap();

        let name = schema.get_field("name").unwrap();
        let name_ng = schema.get_field("name_ng").unwrap();
        let desc = schema.get_field("desc").unwrap();
        let t = schema.get_field("mtype").unwrap();
        let qp = QueryParser::for_index(&index, vec![name, name_ng, desc]);

        Self {
            options,
            index,
            reader,
            qp,
        }
    }

    fn do_query(
        &self,
        _col: &str,
        qs: &str,
        limit: usize,
    ) -> Result<Vec<Document>, index::IndexError> {
        let searcher = self.reader.searcher();
        let mut results = Vec::new();

        trace!("do_query: {}", qs);

        match self.qp.parse_query(qs) {
            Ok(query) => match searcher.search(&query, &TopDocs::with_limit(limit)) {
                Ok(res) => {
                    for (_score, doc_address) in res {
                        if let Ok(doc) = searcher.doc(doc_address) {
                            results.push(doc);
                        }
                    }
                    debug!("do_query got {} results", results.len());
                    Ok(results)
                }
                Err(err) => {
                    error!("search error: {}", err);
                    Err(index::IndexError::ProcessingError)
                }
            },
            Err(e) => {
                error!("query parse error: {}", e);
                Err(index::IndexError::ProcessingError)
            }
        }
    }
}

impl Default for TantivyOptions {
    fn default() -> Self {
        Self {
            index_dir: String::from("./index/"),
            rebuild: false,
        }
    }
}

/// Converts something implementing the Index trait into a searchable tantivy document.
fn idx_to_doc<T: index::Index>(schema: &Schema, idx: &Box<T>) -> Document {
    let mut doc = Document::default();
    for (j, t) in idx.tuples().iter().enumerate() {
        if j == 0 {
            let id_field = schema.get_field("id").unwrap();
            doc.add_text(id_field, &t.2);
        }
        if t.1 == "name" {
            let f = schema.get_field("name_ng").unwrap();
            doc.add_text(f, &t.3);
        }
        let field = schema.get_field(&t.1).unwrap();
        doc.add_text(field, &t.3);
    }
    let bytes = hex::encode(idx.to_bytes());
    trace!("indexing doc of length {}", bytes.len());
    doc.add_text(schema.get_field("mtype").unwrap(), &idx.mtype());
    doc.add_text(schema.get_field("doc").unwrap(), &bytes);
    doc
}

impl index::Indexer for Tantivy {
    fn index<T: index::Index>(&self, idx: Box<T>) -> Result<(), index::IndexError> {
        let _writer = self.index.writer(500);
        Ok(())
    }

    fn index_bulk<T: index::Index>(&self, curs: Vec<Box<T>>) -> Result<(), index::IndexError> {
        let mut writer = self.index.writer(50_000_000).unwrap();
        let schema = self.index.schema();
        for (i, idx) in curs.iter().enumerate() {
            writer.add_document(idx_to_doc(&schema, idx));

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

    /// Queries tantivy for document byte arrays
    fn query(&self, col: &str, qs: &str) -> Result<Vec<(String, Vec<u8>)>, index::IndexError> {
        let docs = self.do_query(col, qs, RESULT_LIMIT)?;
        let mut r = Vec::new();
        let raw = self.index.schema().get_field("doc").unwrap();
        let mtype = self.index.schema().get_field("mtype").unwrap();
        for doc in docs {
            if let Some(val) = doc.get_first(raw) {
                if let Some(t) = doc.get_first(mtype) {
                    r.push((
                        t.text().unwrap().into(),
                        hex::decode(val.text().unwrap()).unwrap(),
                    ));
                } else {
                    error!("failed to get mtype field");
                }
            } else {
                error!("failed to get raw field");
            }
        }
        Ok(r)
    }

    /// Queries tantivy for matching ids
    fn query_ids(&self, col: &str, qs: &str) -> Result<Vec<String>, index::IndexError> {
        let docs = self.do_query(col, qs, RESULT_LIMIT)?;

        let mut ids = Vec::new();
        let id = self.index.schema().get_field("id").unwrap();
        for doc in docs {
            let val = doc.get_first(id).unwrap();
            ids.push(String::from(val.text().unwrap()));
        }
        Ok(ids)
    }

    fn flush_all(&self, col: &str) -> Result<(), index::IndexError> {
        Ok(())
    }
}
