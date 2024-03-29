mod component;
mod fetch;
mod params;
mod pushshift;

use std::str::FromStr;

use component::search_box::SearchBox;
use component::search_button::{SearchButton, SearchState};
use component::select::Select;
use component::Width;
use fetch::fetch;
use params::{Endpoint, SearchParams};
use pushshift::RedditType;
use time::{format_description, PrimitiveDateTime, UtcOffset};
use url::Url;
use yew::prelude::*;

#[derive(Debug)]
pub enum FetchState {
    NotFetching,
    Fetching,
    Success(Vec<RedditType>, SearchType, SearchParams),
    Done,
    Failed(String),
}

#[derive(Debug)]
enum Msg {
    Search,
    More,
    SetPsFetchState(FetchState),
    UpdateEndpoint(String),
    UpdateSubreddit(String),
    UpdateAuthor(String),
    UpdateQuery(String),
    UpdateTimeStart(String),
    UpdateTimeEnd(String),
}

struct Model {
    results: Vec<RedditType>,
    state: FetchState,
    tz_offset: i64,
    params: SearchParams,
    // For use when "more-ing"
    last_params: Option<SearchParams>,
}

#[derive(Clone, Debug)]
pub enum SearchType {
    Initial,
    More,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        // Get current timezone offset
        let tz_offset = -js_sys::Date::new_0().get_timezone_offset() as i64;

        // Create model
        Self {
            results: Vec::new(),
            state: FetchState::NotFetching,
            tz_offset,
            params: SearchParams::load(),
            last_params: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Search => {
                self.results.clear();
                self.search(ctx, SearchType::Initial);
                false
            }
            Msg::More => {
                self.search(ctx, SearchType::More);
                false
            }
            Msg::UpdateEndpoint(s) => {
                if let Ok(e) = Endpoint::from_str(&s) {
                    self.params.endpoint = e;
                }
                false
            }
            Msg::UpdateSubreddit(s) => {
                self.params.subreddit = s;
                false
            }
            Msg::UpdateQuery(s) => {
                self.params.query = s;
                false
            }
            Msg::UpdateAuthor(s) => {
                self.params.author = s;
                false
            }
            Msg::UpdateTimeStart(s) => {
                self.params.time_start = s;
                false
            }
            Msg::UpdateTimeEnd(s) => {
                self.params.time_end = s;
                false
            }
            Msg::SetPsFetchState(x) => {
                self.params.store();

                if let FetchState::Success(_, _, ref params) = x {
                    // Update last search params
                    self.last_params = Some(params.clone());
                }

                match x {
                    FetchState::Success(r, SearchType::Initial, _) => {
                        self.results = r;
                        self.state = FetchState::Done;
                    }
                    FetchState::Success(mut r, SearchType::More, _) => {
                        self.results.append(&mut r);
                        self.state = FetchState::Done;
                    }
                    _ => self.state = x,
                }
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        // Search box
        let mut elems = vec![self.search_form(ctx)];

        // Results
        if !self.results.is_empty() {
            elems.push(html! {
                <div class="results">
                    {for self.results.iter().map(|r| r.html()).chain(std::iter::once(self.more_button(ctx)))}
                </div>
            });
        }

        // Error message
        if let FetchState::Failed(err) = &self.state {
            elems.push(html! {
                <div class="error">
                    {err}
                </div>
            });
        }

        html! {
            <div>{ for elems.into_iter() }</div>
        }
    }
}

impl Model {
    fn search_form(&self, ctx: &Context<Self>) -> Html {
        let on_endpoint_change = ctx.link().callback(Msg::UpdateEndpoint);
        let on_subreddit_change = ctx.link().callback(Msg::UpdateSubreddit);
        let on_author_change = ctx.link().callback(Msg::UpdateAuthor);
        let on_query_change = ctx.link().callback(Msg::UpdateQuery);
        let on_time_start_change = ctx.link().callback(Msg::UpdateTimeStart);
        let on_time_end_change = ctx.link().callback(Msg::UpdateTimeEnd);
        let on_submit = ctx.link().callback(|e: FocusEvent| {
            e.prevent_default();
            Msg::Search
        });

        let search_state = if matches!(self.state, FetchState::Fetching) {
            SearchState::Working("Fetching...".to_string())
        } else {
            SearchState::Idle("Search".to_string())
        };

        html! {
            <form class="search" onsubmit={on_submit}>
                <input type="submit" style="display: none" />

                <div>
                    <Select width={Width::Half}
                        id={"endpoint"}
                        class={"endpoint"}
                        label={"Endpoint:"}
                        on_input={on_endpoint_change}
                        options={Endpoint::list()}
                        selected={self.params.endpoint.to_string()} />
                </div>

                <div>
                    <SearchBox width={Width::Half}
                        id={"subreddit"}
                        label={"Subreddit:"}
                        on_change={on_subreddit_change}
                        value={self.params.subreddit.clone()} />
                    <div class="spacer" />
                    <SearchBox width={Width::Half}
                        id={"author"}
                        label={"Author:"}
                        on_change={on_author_change}
                        value={self.params.author.clone()} />
                </div>

                <div>
                    <SearchBox width={Width::Half}
                        id={"time_start"}
                        label={"After:"}
                        on_change={on_time_start_change}
                        value={self.params.time_start.clone()} />
                    <div class="spacer" />
                    <SearchBox width={Width::Half}
                        id={"time_end"}
                        label={"Before:"}
                        on_change={on_time_end_change}
                        value={self.params.time_end.clone()} />
                </div>

                <div>
                    <SearchBox width={Width::Full}
                        id={"query"}
                        label={"Query:"}
                        on_change={on_query_change}
                        value={self.params.query.clone()} />
                </div>

                <SearchButton state={search_state} />

                <script src={"bundle.js"}></script>
            </form>
        }
    }

    fn more_button(&self, ctx: &Context<Self>) -> Html {
        let on_click = ctx.link().callback(|_| Msg::More);
        let state = if matches!(self.state, FetchState::Fetching) {
            SearchState::Working("Fetching...".to_string())
        } else {
            SearchState::Idle("More".to_string())
        };

        html! {
            <SearchButton {state} {on_click} />
        }
    }

    fn search(&mut self, ctx: &Context<Self>, search_type: SearchType) {
        let params = match search_type {
            SearchType::Initial => self.params.clone(),
            SearchType::More => self.last_params.clone().unwrap(),
        };

        let url = {
            let mut url = Url::parse(self.params.endpoint.url()).unwrap();

            // Add GET query parameters
            url.query_pairs_mut()
                .append_pair("limit", "1000");

            if !self.params.subreddit.is_empty() {
                url.query_pairs_mut()
                    .append_pair("subreddit", &params.subreddit);
            }

            if !self.params.author.is_empty() {
                url.query_pairs_mut().append_pair("author", &params.author);
            }

            if !self.params.query.is_empty() {
                url.query_pairs_mut().append_pair("q", &params.query);
            }

            if let Some(ts) = parse_time(&params.time_start, self.tz_offset) {
                url.query_pairs_mut().append_pair("after", &ts.to_string());
            }

            // If getting more posts/comments, add "before_id" GET parameter
            if matches!(search_type, SearchType::More) {
                if let Some(r) = self.results.last() {
                    url.query_pairs_mut()
                        .append_pair("before", &r.time().to_string());
                }
            } else if let Some(ts) = parse_time(&params.time_end, self.tz_offset) {
                url.query_pairs_mut().append_pair("before", &ts.to_string());
            }

            url.to_string()
        };

        // Message to send when search finishes
        {
            let tz_offset = self.tz_offset;
            let endpoint = self.params.endpoint.clone();
            ctx.link().send_future(async move {
                match fetch(url).await {
                    Ok(x) => match endpoint.parse(x, tz_offset) {
                        Ok(p) => Msg::SetPsFetchState(FetchState::Success(p, search_type, params)),
                        Err(e) => Msg::SetPsFetchState(FetchState::Failed(e.to_string())),
                    },
                    Err(e) => Msg::SetPsFetchState(FetchState::Failed(e.to_string())),
                }
            });
        }

        ctx.link()
            .send_message(Msg::SetPsFetchState(FetchState::Fetching));
    }
}

fn parse_time(s: impl AsRef<str>, offset: i64) -> Option<i64> {
    let format = format_description::parse("[year]-[month]-[day] [hour]:[minute]").unwrap();
    let ts = PrimitiveDateTime::parse(s.as_ref(), &format)
        .ok()?
        .assume_offset(UtcOffset::from_whole_seconds(60 * offset as i32).ok()?)
        .unix_timestamp();
    Some(ts)
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<Model>();
}
