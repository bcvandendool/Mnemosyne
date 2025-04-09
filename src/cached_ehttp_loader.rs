use directories::ProjectDirs;
use egui::ahash::HashMap;
use egui::{
    load::{Bytes, BytesLoadResult, BytesLoader, BytesPoll, LoadError},
    mutex::Mutex,
};
use std::io::Write;
use std::path::PathBuf;
use std::{fs, sync::Arc, task::Poll, thread};
use urlencoding::decode;
// Cached ehttp loader based on the ehttp loader by emilk for egui:
// https://github.com/emilk/egui/blob/master/crates/egui_extras/src/loaders/ehttp_loader.rs

#[derive(Clone)]
struct File {
    bytes: Arc<[u8]>,
    mime: Option<String>,
}

impl File {
    fn from_response(uri: &str, response: ehttp::Response) -> Result<Self, String> {
        if !response.ok {
            match response.text() {
                Some(response_text) => {
                    return Err(format!(
                        "failed to load {uri:?}: {} {} {response_text}",
                        response.status, response.status_text
                    ));
                }
                None => {
                    return Err(format!(
                        "failed to load {uri:?}: {} {}",
                        response.status, response.status_text
                    ));
                }
            }
        }

        let mime = response.content_type().map(|v| v.to_owned());
        let bytes = response.bytes.into();

        Ok(Self { bytes, mime })
    }
}

type Entry = Poll<Result<File, String>>;

#[derive(Default)]
pub struct CachedEhttpLoader {
    cache: Arc<Mutex<HashMap<String, Entry>>>,
}

impl CachedEhttpLoader {
    pub const ID: &'static str = egui::generate_loader_id!(CachedEhttpLoader);
}

const PROTOCOLS: &[&str] = &["http://", "https://"];

fn starts_with_one_of(s: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| s.starts_with(prefix))
}

impl BytesLoader for CachedEhttpLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str) -> BytesLoadResult {
        if !starts_with_one_of(uri, PROTOCOLS) {
            return Err(LoadError::NotSupported);
        }

        let mut cache = self.cache.lock();
        if let Some(entry) = cache.get(uri).cloned() {
            match entry {
                Poll::Ready(Ok(file)) => Ok(BytesPoll::Ready {
                    size: None,
                    bytes: Bytes::Shared(file.bytes),
                    mime: file.mime,
                }),
                Poll::Ready(Err(err)) => Err(LoadError::Loading(err)),
                Poll::Pending => Ok(BytesPoll::Pending { size: None }),
            }
        } else {
            log::trace!("started loading {uri:?}");

            let uri = uri.to_owned();
            cache.insert(uri.clone(), Poll::Pending);
            drop(cache);

            let project_dirs = ProjectDirs::from("", "", "Mnemosyne").unwrap();
            let mut cache_path = PathBuf::new();
            cache_path.push(project_dirs.cache_dir());
            cache_path.push("boxart/");
            cache_path.push(
                decode(uri.rsplit_once('/').unwrap().1)
                    .unwrap_or_default()
                    .to_string(),
            );

            if fs::exists(&cache_path).unwrap_or(false) {
                thread::Builder::new()
                    .name(format!("egui_extras::FileLoader::load({uri:?})"))
                    .spawn({
                        let ctx = ctx.clone();
                        let cache = self.cache.clone();
                        let uri = uri.to_owned();
                        move || {
                            let result = match std::fs::read(&cache_path) {
                                Ok(bytes) => {
                                    let mime = mime_guess2::from_path(&cache_path)
                                        .first_raw()
                                        .map(|v| v.to_owned());

                                    Ok(File {
                                        bytes: bytes.into(),
                                        mime,
                                    })
                                }
                                Err(err) => Err(err.to_string()),
                            };
                            let prev = cache.lock().insert(uri.clone(), Poll::Ready(result));
                            assert!(matches!(prev, Some(Poll::Pending)), "unexpected state");
                            ctx.request_repaint();
                            log::trace!("finished loading cached {uri:?}");
                        }
                    })
                    .expect("failed to spawn thread");
            } else {
                ehttp::fetch(ehttp::Request::get(uri.clone()), {
                    let ctx = ctx.clone();
                    let cache = self.cache.clone();
                    move |response| {
                        let result = match response {
                            Ok(response) => File::from_response(&uri, response),
                            Err(err) => {
                                // Log details; return summary
                                log::error!("Failed to load {uri:?}: {err}");
                                Err(format!("Failed to load {uri:?}"))
                            }
                        };

                        if result.is_ok() {
                            let cache_file = fs::File::create(&cache_path);
                            cache_file
                                .unwrap()
                                .write_all(&result.as_ref().unwrap().bytes)
                                .expect("Failed to cache image");
                        }

                        log::trace!("finished loading {uri:?}");
                        cache.lock().insert(uri, Poll::Ready(result));
                        ctx.request_repaint();
                    }
                });
            }

            Ok(BytesPoll::Pending { size: None })
        }
    }

    fn forget(&self, uri: &str) {
        let _ = self.cache.lock().remove(uri);
    }

    fn forget_all(&self) {
        self.cache.lock().clear();
    }

    fn byte_size(&self) -> usize {
        self.cache
            .lock()
            .values()
            .map(|entry| match entry {
                Poll::Ready(Ok(file)) => {
                    file.bytes.len() + file.mime.as_ref().map_or(0, |m| m.len())
                }
                Poll::Ready(Err(err)) => err.len(),
                _ => 0,
            })
            .sum()
    }
}
