use crate::get_with_retry;
use crate::{ratelimiter::DeviationRateLimiter, BANNED_URL_LIST};
use crate::{DATA_DESTINATION_DIR, REQUEST_RETRY_COUNT, RETRY_REQUEST_WAIT_DURATION};
use reqwest::Client;
use scraper::ElementRef;
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct MediaData {
  pub url: String,
  pub extension: String,
}

impl MediaData {
  pub async fn download(
    self,
    client: &Client,
    rate_limiter: &DeviationRateLimiter,
    thread_id: &str,
    post_id: &str,
    file_appender: &str,
  ) -> anyhow::Result<()> {
    let MediaData {
      url: media_url,
      extension: media_extension,
    } = self;

    tracing::info!("{thread_id}-{post_id}: Found a media URL.",);

    let media_file_path = format!(
      "{}/{}/{}-{}-{}.{}",
      DATA_DESTINATION_DIR, thread_id, thread_id, post_id, file_appender, media_extension,
    );
    let media_file_path = Path::new(&media_file_path);

    if media_file_path.exists() {
      tracing::info!("{thread_id}-{post_id}: media file already exists.");
      return Ok(());
    }

    if let Some(media_thread_path) = Path::new(&media_file_path).parent() {
      if !media_thread_path.exists() {
        tracing::info!("{thread_id}: Creating directories for media");
        fs::create_dir_all(media_thread_path)?;
      }
    }

    tracing::info!("{thread_id}-{post_id}: Grabbing media URL `{media_url:?}`",);
    let response = get_with_retry(
      client,
      media_url,
      REQUEST_RETRY_COUNT,
      rate_limiter,
      RETRY_REQUEST_WAIT_DURATION,
    )
    .await?;
    let response_bytes = response.bytes().await?;

    tracing::info!("{thread_id}-{post_id}: Obtaining write handle on files.");
    let mut sorted_file = fs::OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(true)
      .open(media_file_path)?;

    tracing::info!("{thread_id}-{post_id}: Writing media bytes to file.");
    sorted_file.write_all(&response_bytes)?;

    Ok(())
  }
}

pub fn extract_hyperlinks_from_post(post: &ElementRef) -> Option<Vec<String>> {
  let mut hyperlinks = vec![];

  let post_wrapper_element = find_child_with_class(post, "post_wrapper")?;
  let text_element = find_child_with_class(&post_wrapper_element, "text")?;

  for child in text_element.child_elements() {
    if child
      .value()
      .has_class("backlink", scraper::CaseSensitivity::CaseSensitive)
    {
      continue;
    }

    let Some(hyperlink) = child.value().attr("href") else {
      continue;
    };

    if BANNED_URL_LIST
      .iter()
      .any(|banned_url| hyperlink.to_lowercase().contains(banned_url))
    {
      continue;
    }

    hyperlinks.push(hyperlink.to_string());
  }

  Some(hyperlinks)
}

pub fn extract_media_url_from_post(post: &ElementRef) -> Option<MediaData> {
  if !post
    .value()
    .has_class("has_image", scraper::CaseSensitivity::CaseSensitive)
  {
    return None;
  }

  let post_wrapper_element = find_child_with_class(post, "post_wrapper")?;
  let post_file_element = find_child_with_class(&post_wrapper_element, "post_file")?;
  let post_file_filename_element = find_child_with_class(&post_file_element, "post_file_filename")?;

  let post_file_filename_value = post_file_filename_element.value();

  let media_url = post_file_filename_value.attr("href").map(str::to_string)?;
  let media_name = post_file_filename_value.attr("title").map(str::to_string)?;

  let media_extension = media_name.split('.').last().map(str::to_string)?;

  Some(MediaData {
    url: media_url,
    extension: media_extension,
  })
}

pub fn find_child_with_class<'a>(element: &'a ElementRef, class: &str) -> Option<ElementRef<'a>> {
  element.child_elements().find(|child| {
    child
      .value()
      .has_class(class, scraper::CaseSensitivity::CaseSensitive)
  })
}
