# Historical blog URL inventory

Recorded: 5 October 2026. This is an inventory, not a redirect or restoration plan.

Source: commit [`7d872a79c75999df718dbb50578ffe16b7b09fc5`](https://github.com/MrCoder/mrcoder.github.io/tree/7d872a79c75999df718dbb50578ffe16b7b09fc5), the parent of `6306d08`. Its `_posts/` contains twenty posts from 2013–2017. The current skills site had already replaced these files before the redesign.

These are source-derived historical paths. They have not been confirmed against an archived production crawl. The pinned [`_config.yml`](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_config.yml) uses `/:categories/:title/`. [`Gemfile.lock`](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/Gemfile.lock) pins Jekyll 3.1.6. Its [URL placeholder code](https://github.com/jekyll/jekyll/blob/v3.1.6/lib/jekyll/drops/url_drop.rb) lowercases categories but preserves filename slug casing for `:title`; [URL sanitization](https://github.com/jekyll/jekyll/blob/v3.1.6/lib/jekyll/url.rb) collapses the empty category segment. That is why `Windows` remains capitalized in the final post path, while `Tech`, `Database`, and `Kafka` become lowercase.

| Historical path | Post | Evidence |
| --- | --- | --- |
| `/build-a-site-with-nodejs-express-mongodb-and-angularjs0/` | Build a Site with NodeJs, Express, MongoDB and AngularJs(0) | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2013-04-22-build-a-site-with-nodejs-express-mongodb-and-angularjs0.md) |
| `/articles-read-april-2013/` | Articles Read (April 2013) | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2013-04-24-articles-read-april-2013.md) |
| `/tech/articles-read-in-may-2013/` | Articles Read in May 2013 | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2013-04-29-articles-read-in-may-2013.md) |
| `/tech/build-nanki-1/` | Build Nanki (1) | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2013-04-29-build-nanki-1.md) |
| `/tech/what-is-nosql/` | What is NoSQL | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2013-04-29-what-is-nosql.md) |
| `/comparing-scala-with-javascript/` | Comparing Scala with JavaScript | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2013-04-30-comparing-scala-with-javascript.md) |
| `/understanding-require-in-nodejs/` | Understanding Require In NodeJs | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2013-05-05-understanding-require-in-nodejs.md) |
| `/articles-read-in-june/` | Articles Read in June | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2013-06-19-articles-read-in-june.md) |
| `/articles-read-in-july-2013/` | Articles Read in July 2013 | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2013-07-03-articles-read-in-july-2013.md) |
| `/code-highlighting-post/` | Syntax Highlighting Post | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2013-08-16-code-highlighting-post.md) |
| `/accessibility-shiv-with-angularjs/` | Accessibility shiv with AngularJs | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2013-09-29-accessibility-shiv-with-angularjs.md) |
| `/background-image/` | Post with a Background Image | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2013-10-26-background-image.md) |
| `/stop-mocking-start-newing/` | Stop mocking, start newing | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2015-03-26-stop-mocking-start-newing.md) |
| `/reactjs-for-angular-developer/` | ReactJs for angularJs developer | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2015-04-15-reactjs-for-angular-developer.md) |
| `/reactjs-progress-bar-component/` | ReactJs progress bar component and wrap it in an Angular directive | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2015-04-27-reactjs-progress-bar-component.md) |
| `/dispatcher-in-the-flux-architecture/` | Dispatcher in the flux architecture | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2015-05-01-dispatcher-in-the-flux-architecture.md) |
| `/run-jekyll-with-docker/` | Run Jekyll in Docker on Mac OSX | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2015-06-06-run-jekyll-with-docker.md) |
| `/database/architects-diary-tablespace/` | 20170627 Architect's Diary - Tablespace | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2017-06-27-architects-diary-tablespace.md) |
| `/database/architects-diary-sql-command-not-properly-ended/` | 20170627 Architect's Diary - ORA-00933: SQL Command not properly ended | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2017-06-28-architects-diary-sql-command-not-properly-ended.md) |
| `/kafka/architects-diary-install-kafka-on-Windows/` | 20170710 Architect's Diary - Install Kafka on Windows | [source](https://github.com/MrCoder/mrcoder.github.io/blob/7d872a79c75999df718dbb50578ffe16b7b09fc5/_posts/2017-07-10-architects-diary-install-kafka-on-Windows.md) |

Other historical surfaces:

| Path | Source or role | First-release disposition |
| --- | --- | --- |
| `/` | Paginated blog index | Replaced by the skill catalogue before this redesign |
| `/about/` | `about/index.md`, theme information | New Field Guide provenance page at the same path |
| `/posts/` | `posts/index.html`, post archive | No restoration or redirect in this release |
| `/tags/` | `tags/index.html`, tag archive | No restoration or redirect in this release |
| `/theme-setup/` | `theme-setup/index.md`, theme setup | No restoration or redirect in this release |
| `/feed.xml` | Expected Jekyll Feed plugin output; plugin is in the pinned config/lock | No new feed in this release; confirm deployed path before migration |
| `/404.html` | Explicit permalink in `404.md` | New useful static 404 page |

Do not redirect unrelated old articles to the skills catalogue. A future archive review should decide whether each article deserves restoration, a relevant replacement, or continued 404 behavior. Confirm observed URLs and search/backlink evidence before adding redirects; the source inventory alone does not establish current traffic. No historical article has been restored and no redirect has been added by this inventory.
