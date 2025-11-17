use crate::helper_methods::*;
use crate::html_parsing::*;
use ratelimiter::DeviationRateLimiter;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::ops::RangeInclusive;
use std::path::Path;
use std::time::Duration;
use tracing::level_filters::LevelFilter;

pub mod helper_methods;
pub mod html_parsing;
pub mod ratelimiter;

const THREAD_SEARCH_URL: &str = "https://archive.palanq.win/vt/search/subject/%2Fshon%2F";
const THREAD_PAGE_URL: &str = "https://archive.palanq.win/vt/thread/";
#[allow(unused)]
const DOWNLOAD_PAGES: RangeInclusive<usize> = 1..=52;
const BANNED_URL_LIST: &[&str] = &[
  "x.",
  "twitter.",
  "youtube.",
  "twitch.",
  "youtu.",
  "wikipedia.",
  "steampowered.",
  "amiami.",
  "gov.",
  "gitlab.",
  "github.",
  "fandom.",
  "poal.",
  "spanix",
  "pixiv.",
  "amazon.",
  "gamersupps.",
  "nexusmods.",
  "speedrun.",
  "yle.",
  "amazon.",
];
const DATA_DESTINATION_DIR: &str = "data";
pub const MAX_REQUEST_RATE_LIMIT: u64 = 4;
pub const BASE_RATE_LIMIT_DURATION: Duration = Duration::new(0, 143_240_219);
pub const RETRY_REQUEST_WAIT_DURATION: Duration = Duration::new(0, 51_230_508);
pub const REQUEST_RETRY_COUNT: usize = 5;

#[tokio::main]
async fn main() {
  let file = tracing_appender::rolling::daily("logs/", "archive_scraper.log");

  tracing_subscriber::fmt()
    .with_max_level(LevelFilter::INFO)
    .with_writer(file)
    .with_ansi(false)
    .init();

  let client = Client::new();
  let rate_limiter = DeviationRateLimiter::new().unwrap();

  download_images_from_page_range(&client, &rate_limiter, DOWNLOAD_PAGES).await;
  // download_images_from_file_list(&client, &rate_limiter, "text_data/catbox_urls.txt").await;

  tracing::info!("Process finished!");
}

async fn get_thread_page_id(
  client: &Client,
  rate_limiter: &DeviationRateLimiter,
  page_number: usize,
) -> anyhow::Result<Vec<String>> {
  tracing::info!("Reading page number {}", page_number);

  let page_url = format!("{}/page/{}", THREAD_SEARCH_URL, page_number);
  let response = get_with_retry(
    client,
    page_url,
    REQUEST_RETRY_COUNT,
    rate_limiter,
    RETRY_REQUEST_WAIT_DURATION,
  )
  .await?;

  let response_body = response.text().await?;
  let parsed_response = Html::parse_document(&response_body);

  let article_selector = Selector::parse("article").unwrap();
  let selection = parsed_response.select(&article_selector);

  Ok(
    selection
      .into_iter()
      .map(|element| element.value())
      .map(|element_value| element_value.id())
      .filter_map(|id| id.map(|id| id.to_string()))
      .collect(),
  )
}

async fn download_images_and_urls_of_interest_from_thread(
  client: &Client,
  rate_limiter: &DeviationRateLimiter,
  thread_id: &str,
) -> anyhow::Result<()> {
  tracing::info!("{thread_id}: Requesting thread page.");
  let thread_list_url = format!("{}{}", THREAD_PAGE_URL, thread_id);
  let response = get_with_retry(
    client,
    thread_list_url,
    REQUEST_RETRY_COUNT,
    rate_limiter,
    RETRY_REQUEST_WAIT_DURATION,
  )
  .await?;

  tracing::info!("{thread_id}: Got response.");

  let response_text = response.text().await?;
  let response_html = Html::parse_document(&response_text);

  let post_selector = Selector::parse("article").unwrap();
  let posts = response_html.select(&post_selector);

  for post in posts.into_iter() {
    let post_value = post.value();

    if post_value.has_class("post_is_op", scraper::CaseSensitivity::CaseSensitive)
      || post_value.has_class(
        "backlink_container",
        scraper::CaseSensitivity::CaseSensitive,
      )
    {
      continue;
    }

    let Some(post_id) = post_value.id() else {
      tracing::warn!("Attempted to read an article with a missing ID.\n{post_value:?}",);

      continue;
    };

    if let Some(image_data) = extract_media_url_from_post(&post) {
      image_data
        .download(client, rate_limiter, thread_id, post_id, "")
        .await?;
    }

    if let Some(hyperlinks) = extract_hyperlinks_from_post(&post) {
      if !hyperlinks.is_empty() {
        tracing::info!("{thread_id}-{post_id}: Extracted hyperlinks of interest: {hyperlinks:?}");
        write_hyperlinks_to_disk(hyperlinks, thread_id, post_id).await?;
      }
    }
  }

  Ok(())
}

async fn write_hyperlinks_to_disk(
  hyperlinks: Vec<String>,
  thread_id: &str,
  post_id: &str,
) -> anyhow::Result<()> {
  let file_path = format!("{}/urls.txt", DATA_DESTINATION_DIR);
  let file_path = Path::new(&file_path);

  if let Some(hyperlink_parent_dirs) = Path::new(&file_path).parent() {
    if !hyperlink_parent_dirs.exists() {
      tracing::info!("Creating dir for hyperlinks");
      fs::create_dir_all(hyperlink_parent_dirs)?;
    }
  }

  let mut hyperlink_file = fs::OpenOptions::new()
    .append(true)
    .create(true)
    .truncate(false)
    .open(file_path)?;

  for hyperlink in hyperlinks {
    let line = format!("{}-{}: {}", thread_id, post_id, hyperlink);

    if let Err(error) = writeln!(hyperlink_file, "{}", line) {
      tracing::error!(
        "Failed to write {thread_id}/{post_id} to file. {{{hyperlink}}}. Reason: `{error:?}`",
      );
    }
  }

  Ok(())
}

#[allow(unused)]
async fn download_images_from_page_range(
  client: &Client,
  rate_limiter: &DeviationRateLimiter,
  page_range: RangeInclusive<usize>,
) {
  for page_number in page_range {
    let thread_id_result = get_thread_page_id(client, rate_limiter, page_number).await;
    let thread_ids = match thread_id_result {
      Ok(thread_ids) => thread_ids,
      Err(error) => {
        tracing::error!("Failed to read page number {page_number:?}. Reason: `{error:?}`");

        continue;
      }
    };

    tracing::info!("Got thread ids. Processing {:?}", thread_ids);

    for thread_id in thread_ids {
      if let Err(error) =
        download_images_and_urls_of_interest_from_thread(client, rate_limiter, &thread_id).await
      {
        tracing::error!(
          "An error occurred when attempting to download from thread {thread_id:?}. Reason: `{error:?}`",
        );
      }
    }
  }
}

/// The file is expected to be in the format of `thread_id-post_id: url.extension`.
/// example:
/// ```
/// 58931442-58935074: https://files.catbox.moe/1qk6mu.mp3
/// 46910262-46936051: https://files.catbox.moe/28fgj6.png
/// 63361527-63109637: https://files.catbox.moe/29bg8r.png
/// 48846709-48887663: https://files.catbox.moe/2hogqc.png
/// 73339334-73391757: https://files.catbox.moe/2ifgg7.mp4
/// 55924632-55962850: https://files.catbox.moe/2lhgt8.mp3
/// 80352791-80373537: https://files.catbox.moe/2n9g6f.png
/// 48472611-48561502: https://files.catbox.moe/2otgte.mp4
/// 48700691-48742488: https://files.catbox.moe/2regli.mp4
/// ```
#[allow(unused)]
async fn download_images_from_file_list<P: AsRef<Path>>(
  client: &Client,
  rate_limiter: &DeviationRateLimiter,
  file_path: P,
) {
  let file_path = file_path.as_ref();
  let mut file_list = fs::File::open(file_path).unwrap();
  let mut urls = String::new();
  file_list.read_to_string(&mut urls).unwrap();

  let mut checked_post_ids: HashMap<String, usize> = HashMap::new();
  let url_data: Vec<(String, String, MediaData)> = urls
    .lines()
    .filter_map(|line| {
      let line_data: Vec<&str> = line.splitn(2, ": ").collect();
      let thread_data: Vec<&str> = line_data.first()?.splitn(2, "-").collect();

      let url = line_data.get(1)?.to_string();
      let extension = url.split('.').next_back()?.to_string();
      let image_data = MediaData { url, extension };

      Some((
        thread_data.first()?.to_string(),
        thread_data.get(1)?.to_string(),
        image_data,
      ))
    })
    .collect();

  // TEMP
  let post_ids: Vec<String> = url_data
    .clone()
    .into_iter()
    .map(|(_, post_id, _)| post_id)
    .collect();
  let mut posts_with_frequency: HashMap<String, usize> = HashMap::new();

  post_ids.iter().for_each(|post_id| {
    let counter_entry = posts_with_frequency.entry(post_id.clone()).or_insert(0);
    *counter_entry += 1;
  });

  let duplicate_posts: HashSet<String> = posts_with_frequency
    .into_iter()
    .filter_map(|(post_id, frequency)| (frequency > 1).then_some(post_id))
    .collect();
  // TEMP

  for (thread_id, post_id, image_data) in url_data {
    if !duplicate_posts.contains(&post_id) {
      continue;
    }

    let file_appender = checked_post_ids
      .get(&post_id)
      .map(|count| (*count).to_string())
      .unwrap_or_default();

    if let Err(error) = image_data
      .download(client, rate_limiter, &thread_id, &post_id, &file_appender)
      .await
    {
      tracing::error!(
        "{thread_id:?}-{post_id:?}: Image could not be downloaded. Reason: {error:?}"
      );
    }

    let counter_entry = checked_post_ids.entry(post_id.clone()).or_insert(1);
    *counter_entry += 1;
  }
}
