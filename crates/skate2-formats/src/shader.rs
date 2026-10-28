//! Named render-effect manifests stored in `render.big`.

use std::{fmt, str};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderManifest {
    pub version: String,
    pub vertex_programs: Vec<ShaderProgram>,
    pub pixel_programs: Vec<ShaderProgram>,
    pub techniques: Vec<ShaderTechnique>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderProgram {
    pub id: u32,
    pub size: u32,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderTechnique {
    pub name: String,
    pub passes: Vec<ShaderPass>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderPass {
    pub id: u32,
    pub vertex_shader: u32,
    pub pixel_shader: u32,
    pub states: Vec<NamedShaderState>,
    pub samplers: Vec<ShaderSampler>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShaderSampler {
    pub name: String,
    pub states: Vec<NamedShaderState>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamedShaderState {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShaderManifestError {
    InvalidUtf8,
    MalformedTag,
    MissingAttribute {
        tag: String,
        attribute: &'static str,
    },
    InvalidInteger {
        tag: String,
        attribute: &'static str,
    },
    UnexpectedTag(String),
    UnexpectedNesting(String),
    MissingShaderReference {
        pass: u32,
        stage: &'static str,
    },
    DeclaredCountMismatch {
        category: &'static str,
        declared: u32,
        actual: usize,
    },
    UnclosedElement(&'static str),
}

impl fmt::Display for ShaderManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid shader manifest: {self:?}")
    }
}

impl std::error::Error for ShaderManifestError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProgramSection {
    None,
    Vertex,
    Pixel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeclaredCounts {
    vertex: u32,
    pixel: u32,
    techniques: u32,
}

#[derive(Debug)]
struct OpenPass {
    id: u32,
    vertex_shader: Option<u32>,
    pixel_shader: Option<u32>,
    states: Vec<NamedShaderState>,
    samplers: Vec<ShaderSampler>,
}

#[derive(Debug)]
struct Tag {
    name: String,
    attributes: Vec<(String, String)>,
    closing: bool,
}

impl Tag {
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find_map(|(candidate, value)| (candidate == name).then_some(value.as_str()))
    }

    fn required(&self, attribute: &'static str) -> Result<&str, ShaderManifestError> {
        self.attribute(attribute)
            .ok_or_else(|| ShaderManifestError::MissingAttribute {
                tag: self.name.clone(),
                attribute,
            })
    }

    fn integer(&self, attribute: &'static str) -> Result<u32, ShaderManifestError> {
        self.required(attribute)?
            .parse()
            .map_err(|_| ShaderManifestError::InvalidInteger {
                tag: self.name.clone(),
                attribute,
            })
    }
}

#[derive(Debug)]
struct ManifestParser {
    version: Option<String>,
    counts: Option<DeclaredCounts>,
    vertex_programs: Vec<ShaderProgram>,
    pixel_programs: Vec<ShaderProgram>,
    techniques: Vec<ShaderTechnique>,
    section: ProgramSection,
    technique: Option<ShaderTechnique>,
    pass: Option<OpenPass>,
    sampler: Option<ShaderSampler>,
}

impl ManifestParser {
    fn new() -> Self {
        Self {
            version: None,
            counts: None,
            vertex_programs: Vec::new(),
            pixel_programs: Vec::new(),
            techniques: Vec::new(),
            section: ProgramSection::None,
            technique: None,
            pass: None,
            sampler: None,
        }
    }

    fn process(&mut self, tag: Tag) -> Result<(), ShaderManifestError> {
        if tag.closing {
            self.close(tag)
        } else {
            self.open(tag)
        }
    }

    fn close(&mut self, tag: Tag) -> Result<(), ShaderManifestError> {
        match tag.name.as_str() {
            "File" | "Program" => {}
            "VertexProgram" | "PixelProgram" => self.section = ProgramSection::None,
            "Sampler" => {
                let value = self
                    .sampler
                    .take()
                    .ok_or_else(|| ShaderManifestError::UnexpectedNesting(tag.name.clone()))?;
                current_pass(&mut self.pass, &tag)?.samplers.push(value);
            }
            "Pass" => {
                let value =
                    finish_pass(self.pass.take().ok_or_else(|| {
                        ShaderManifestError::UnexpectedNesting(tag.name.clone())
                    })?)?;
                self.technique
                    .as_mut()
                    .ok_or_else(|| ShaderManifestError::UnexpectedNesting(tag.name.clone()))?
                    .passes
                    .push(value);
            }
            "Technique" => {
                self.techniques.push(
                    self.technique
                        .take()
                        .ok_or_else(|| ShaderManifestError::UnexpectedNesting(tag.name.clone()))?,
                );
            }
            _ => return Err(ShaderManifestError::UnexpectedTag(tag.name)),
        }
        Ok(())
    }

    fn open(&mut self, tag: Tag) -> Result<(), ShaderManifestError> {
        match tag.name.as_str() {
            "File" => self.version = Some(tag.required("version")?.to_owned()),
            "Header" => {
                self.counts = Some(DeclaredCounts {
                    vertex: tag.integer("vbCount")?,
                    pixel: tag.integer("pbCount")?,
                    techniques: tag.integer("tecCount")?,
                });
            }
            "VertexProgram" => self.section = ProgramSection::Vertex,
            "PixelProgram" => self.section = ProgramSection::Pixel,
            "Program" => self.add_program(&tag)?,
            "Technique" => {
                self.technique = Some(ShaderTechnique {
                    name: tag.required("name")?.to_owned(),
                    passes: Vec::new(),
                });
            }
            "Pass" => {
                self.pass = Some(OpenPass {
                    id: tag.integer("id")?,
                    vertex_shader: None,
                    pixel_shader: None,
                    states: Vec::new(),
                    samplers: Vec::new(),
                });
            }
            "VertexShader" => {
                current_pass(&mut self.pass, &tag)?.vertex_shader = Some(tag.integer("index")?);
            }
            "PixelShader" => {
                current_pass(&mut self.pass, &tag)?.pixel_shader = Some(tag.integer("index")?);
            }
            "State" => current_pass(&mut self.pass, &tag)?
                .states
                .push(named_state(&tag)?),
            "Sampler" => {
                self.sampler = Some(ShaderSampler {
                    name: tag.required("name")?.to_owned(),
                    states: Vec::new(),
                });
            }
            "SamplerState" => self
                .sampler
                .as_mut()
                .ok_or_else(|| ShaderManifestError::UnexpectedNesting(tag.name.clone()))?
                .states
                .push(named_state(&tag)?),
            _ => return Err(ShaderManifestError::UnexpectedTag(tag.name)),
        }
        Ok(())
    }

    fn add_program(&mut self, tag: &Tag) -> Result<(), ShaderManifestError> {
        let program = ShaderProgram {
            id: tag.integer("id")?,
            size: tag.integer("size")?,
            name: tag.required("name")?.to_owned(),
        };
        match self.section {
            ProgramSection::Vertex => self.vertex_programs.push(program),
            ProgramSection::Pixel => self.pixel_programs.push(program),
            ProgramSection::None => {
                return Err(ShaderManifestError::UnexpectedNesting(tag.name.clone()));
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<ShaderManifest, ShaderManifestError> {
        if self.sampler.is_some() {
            return Err(ShaderManifestError::UnclosedElement("Sampler"));
        }
        if self.pass.is_some() {
            return Err(ShaderManifestError::UnclosedElement("Pass"));
        }
        if self.technique.is_some() {
            return Err(ShaderManifestError::UnclosedElement("Technique"));
        }
        let counts = self
            .counts
            .ok_or_else(|| ShaderManifestError::MissingAttribute {
                tag: "File".to_owned(),
                attribute: "Header",
            })?;
        validate_count("vertex programs", counts.vertex, self.vertex_programs.len())?;
        validate_count("pixel programs", counts.pixel, self.pixel_programs.len())?;
        validate_count("techniques", counts.techniques, self.techniques.len())?;
        Ok(ShaderManifest {
            version: self
                .version
                .ok_or_else(|| ShaderManifestError::MissingAttribute {
                    tag: "File".to_owned(),
                    attribute: "version",
                })?,
            vertex_programs: self.vertex_programs,
            pixel_programs: self.pixel_programs,
            techniques: self.techniques,
        })
    }
}

impl ShaderManifest {
    /// Parses the compact XML effect description used by the reference shader
    /// system. Program bytecode remains an external named asset.
    ///
    /// # Errors
    ///
    /// Returns [`ShaderManifestError`] for malformed XML, invalid nesting,
    /// missing pass bindings, or header/count disagreement.
    pub fn parse(bytes: &[u8]) -> Result<Self, ShaderManifestError> {
        let text = str::from_utf8(bytes).map_err(|_| ShaderManifestError::InvalidUtf8)?;
        let mut parser = ManifestParser::new();
        for tag in parse_tags(text)? {
            parser.process(tag)?;
        }
        parser.finish()
    }
}

fn current_pass<'a>(
    pass: &'a mut Option<OpenPass>,
    tag: &Tag,
) -> Result<&'a mut OpenPass, ShaderManifestError> {
    pass.as_mut()
        .ok_or_else(|| ShaderManifestError::UnexpectedNesting(tag.name.clone()))
}

fn named_state(tag: &Tag) -> Result<NamedShaderState, ShaderManifestError> {
    Ok(NamedShaderState {
        name: tag.required("name")?.to_owned(),
        value: tag.required("value")?.to_owned(),
    })
}

fn finish_pass(pass: OpenPass) -> Result<ShaderPass, ShaderManifestError> {
    Ok(ShaderPass {
        id: pass.id,
        vertex_shader: pass
            .vertex_shader
            .ok_or(ShaderManifestError::MissingShaderReference {
                pass: pass.id,
                stage: "vertex",
            })?,
        pixel_shader: pass
            .pixel_shader
            .ok_or(ShaderManifestError::MissingShaderReference {
                pass: pass.id,
                stage: "pixel",
            })?,
        states: pass.states,
        samplers: pass.samplers,
    })
}

fn validate_count(
    category: &'static str,
    declared: u32,
    actual: usize,
) -> Result<(), ShaderManifestError> {
    if usize::try_from(declared).ok() == Some(actual) {
        Ok(())
    } else {
        Err(ShaderManifestError::DeclaredCountMismatch {
            category,
            declared,
            actual,
        })
    }
}

fn parse_tags(text: &str) -> Result<Vec<Tag>, ShaderManifestError> {
    let mut tags = Vec::new();
    let mut remainder = text;
    while let Some(start) = remainder.find('<') {
        remainder = &remainder[start + 1..];
        let end = remainder
            .find('>')
            .ok_or(ShaderManifestError::MalformedTag)?;
        let mut body = remainder[..end].trim();
        remainder = &remainder[end + 1..];
        if body.starts_with('?') || body.starts_with('!') {
            continue;
        }
        let closing = body.starts_with('/');
        if closing {
            body = body[1..].trim();
        }
        if body.ends_with('/') {
            body = body[..body.len() - 1].trim_end();
        }
        let name_end = body.find(char::is_whitespace).unwrap_or(body.len());
        if name_end == 0 {
            return Err(ShaderManifestError::MalformedTag);
        }
        tags.push(Tag {
            name: body[..name_end].to_owned(),
            attributes: parse_attributes(&body[name_end..])?,
            closing,
        });
    }
    Ok(tags)
}

fn parse_attributes(mut text: &str) -> Result<Vec<(String, String)>, ShaderManifestError> {
    let mut attributes = Vec::new();
    while !text.trim_start().is_empty() {
        text = text.trim_start();
        let equals = text.find('=').ok_or(ShaderManifestError::MalformedTag)?;
        let name = text[..equals].trim_end();
        text = text[equals + 1..].trim_start();
        if name.is_empty() || !text.starts_with('"') {
            return Err(ShaderManifestError::MalformedTag);
        }
        text = &text[1..];
        let end = text.find('"').ok_or(ShaderManifestError::MalformedTag)?;
        attributes.push((name.to_owned(), text[..end].to_owned()));
        text = &text[end + 1..];
    }
    Ok(attributes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_named_programs_techniques_states_and_samplers() {
        let xml = br#"<File version="0.01.0">
<Header vbCount="1" pbCount="1" tecCount="1"/>
<VertexProgram><Program id="0" size="16" name="world.vpo"></Program></VertexProgram>
<PixelProgram><Program id="1" size="24" name="world.fpo"></Program></PixelProgram>
<Technique name="default"><Pass id="0">
<VertexShader index="0"/><PixelShader index="1"/>
<State name="CULLMODE" value="FRONT"/>
<Sampler name="i_diffuse"><SamplerState name="MINFILTER" value="ANISOTROPIC"/></Sampler>
</Pass></Technique></File>"#;
        let manifest = ShaderManifest::parse(xml).unwrap();
        assert_eq!(manifest.version, "0.01.0");
        assert_eq!(manifest.vertex_programs[0].name, "world.vpo");
        assert_eq!(manifest.pixel_programs[0].size, 24);
        assert_eq!(manifest.techniques[0].passes[0].states[0].value, "FRONT");
        assert_eq!(
            manifest.techniques[0].passes[0].samplers[0].name,
            "i_diffuse"
        );
    }

    #[test]
    fn rejects_declared_count_disagreement() {
        let xml =
            br#"<File version="0.01.0"><Header vbCount="1" pbCount="0" tecCount="0"/></File>"#;
        assert!(matches!(
            ShaderManifest::parse(xml),
            Err(ShaderManifestError::DeclaredCountMismatch {
                category: "vertex programs",
                declared: 1,
                actual: 0,
            })
        ));
    }
}
