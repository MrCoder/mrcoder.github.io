import mixpanel from "https://cdn.mxpnl.com/libs/mixpanel-js/dist/mixpanel.module.js";

mixpanel.init("62d0ff230c6799db2a4d30a04fe5e1e2", {
  autocapture: false,
  persistence: "localStorage",
  skip_first_touch_marketing: true,
  track_marketing: false,
  track_pageview: false,
  property_blacklist: [
    "$current_url",
    "$initial_referrer",
    "$initial_referring_domain",
    "$referrer",
    "$referring_domain",
  ],
});

mixpanel.track("mrcoder_site_page_viewed", {
  site: "mrcoder.github.io",
  page_path: window.location.pathname,
});
