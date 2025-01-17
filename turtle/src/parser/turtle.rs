//! Adapter for the Turtle parser from [RIO](https://github.com/Tpt/rio/blob/master/turtle/src/turtle.rs)

use std::io::BufRead;

use rio_api::parser::ParseError;
use rio_turtle::{TurtleError, TurtleParser as RioTurtleParser};
use sophia_api::parser::{Location, TripleParser, WithLocation};
use sophia_rio::parser::*;
use thiserror::Error;

/// Turtle parser based on RIO.
#[derive(Clone, Debug, Default)]
pub struct TurtleParser {
    /// The base IRI used by this parser to resolve relative IRI-references.
    pub base: Option<String>,
}

impl<B: BufRead> TripleParser<B> for TurtleParser {
    type Source = StrictRioSource<RioTurtleParser<B>, TurtleError>;
    fn parse(&self, data: B) -> Self::Source {
        // TODO issue TurtleError if base can not be parsed
        let base = self.base.clone().and_then(|b| oxiri::Iri::parse(b).ok());
        StrictRioSource::Parser(RioTurtleParser::new(data, base))
    }
}

/// A wrapper around [`rio_turtle::TurtleError`] that implements [`WithLocation`].
#[derive(Debug, Error)]
#[error("{0}")]
pub struct SophiaTurtleError(pub TurtleError);

impl WithLocation for SophiaTurtleError {
    fn location(&self) -> Location {
        match self.0.textual_position() {
            None => Location::Unknown,
            Some(pos) => Location::from_lico(
                pos.line_number() as usize + 1,
                pos.byte_number() as usize + 1,
            ),
        }
    }
}

sophia_api::def_mod_functions_for_bufread_parser!(TurtleParser, TripleParser);

// ---------------------------------------------------------------------------------
//                                      tests
// ---------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use sophia_api::graph::Graph;
    use sophia_api::ns::{rdf, xsd};
    use sophia_api::term::matcher::ANY;
    use sophia_api::triple::stream::TripleSource;
    use sophia_inmem::graph::FastGraph;
    use sophia_term::StaticTerm;

    #[test]
    fn test_simple_turtle_string() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let turtle = r#"
            @prefix : <http://example.org/ns/> .

            <#me> :knows [ a :Person ; :name "Alice" ].
        "#;

        let mut g = FastGraph::new();
        let p = TurtleParser {
            base: Some("http://localhost/ex".into()),
        };
        let c = p.parse_str(turtle).add_to_graph(&mut g)?;
        assert_eq!(c, 3);
        assert!(g
            .triples_matching(
                &StaticTerm::new_iri("http://localhost/ex#me").unwrap(),
                &StaticTerm::new_iri("http://example.org/ns/knows").unwrap(),
                &ANY,
            )
            .next()
            .is_some());
        assert!(g
            .triples_matching(
                &ANY,
                &rdf::type_,
                &StaticTerm::new_iri("http://example.org/ns/Person").unwrap(),
            )
            .next()
            .is_some());
        assert!(g
            .triples_matching(
                &ANY,
                &StaticTerm::new_iri("http://example.org/ns/name").unwrap(),
                &StaticTerm::new_literal_dt("Alice", xsd::string).unwrap(),
            )
            .next()
            .is_some());
        Ok(())
    }
}
