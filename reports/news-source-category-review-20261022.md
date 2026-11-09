# News source category review — October 22, 2026

Read-only production review (America/Chicago). No category, source, article or enablement changes were made.

Inventoried all 63 rows in dbo.Sources and joined their current SourceCategories assignments. All 63 were enabled and assigned; all had NULL LastError and stored articles. Reviewed up to five latest stored titles and article URLs per source (315 samples total), plus feed URL scope. Stored article total: 626; Flatland had six and all other sources had ten. This is a category review, not a full endpoint availability or article-body audit.

## Findings

**Clear reassignment:** Awful Announcing (7895), TV/Film → Sports. Its subject is sports media, even when reporting on television. Publisher homepage independently confirms that scope: https://awfulannouncing.com/.

**Optional taxonomy choices:** IGN (1768) and Polygon (1), Entertainment → Video Games if gaming publishers should live together. Both broad feeds also contain film/anime/pop-culture coverage. Keep Nerdist in Entertainment.

**Feed scope rather than category errors:** Guardian (8906) is broad latest content in Top Headlines; Windows Central (3792) mixes games into Technology; THR (1765) mixes lifestyle/music business into TV/Film. Narrow feeds would be needed for strict topic separation. No replacement endpoint was validated by this review.

General News versus Top Headlines needs an editorial definition. Current broad-news assignments are defensible, but neither category is enforced by story-level classification. Keep NPR, CBS News and STLtoday in General News unless the desired policy is to group all national publishers in Top Headlines.

## Current category inventory

| Category | Enabled sources |
| --- | ---: |
| Big 12 News | 5 |
| Box Office | 4 |
| Entertainment | 3 |
| F1 News | 4 |
| General News | 3 |
| Kansas City News | 5 |
| Linux | 5 |
| Men's College Basketball News | 3 |
| Sports | 3 |
| TV/Film | 5 |
| Technology | 10 |
| Top Headlines | 7 |
| Video Games | 6 |

## Per-source assessment

| ID | Source | Current category | Assessment | Evidence / rationale |
| --- | --- | --- | --- | --- |
| 8997 | [Big 12 Conference — All Sports](https://big12sports.com/rss.aspx) | Big 12 News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 8998 | [Heartland College Sports](https://www.heartlandcollegesports.com/feed/) | Big 12 News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 8999 | [Big 12 Conference — Football](https://big12sports.com/rss.aspx?path=football) | Big 12 News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9000 | [Big 12 Conference — Men's Basketball](https://big12sports.com/rss.aspx?path=mbball) | Big 12 News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9001 | [Big 12 Conference — Baseball](https://big12sports.com/rss.aspx?path=baseball) | Big 12 News | Keep | Baseball feed includes multi-sport athlete awards. Older publication dates are plausible during the off-season and do not indicate a category error. |
| 7094 | [Deadline — Box Office](https://deadline.com/v/box-office/feed/) | Box Office | Keep | Publisher box-office tag includes Broadway and related industry/marketing coverage; keep if category means broader box-office industry news. |
| 9034 | [Variety — Box Office](https://variety.com/v/film/box-office/feed/) | Box Office | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9035 | [The Hollywood Reporter — Box Office](https://www.hollywoodreporter.com/t/box-office/feed/) | Box Office | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9036 | [IndieWire — Box Office](https://www.indiewire.com/c/box-office/feed/) | Box Office | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 1 | [Polygon](https://www.polygon.com/) | Entertainment | Consider Video Games | Two of five samples directly concern video games; others cover tabletop, anime and a game adaptation. Mixed entertainment feed, so this is a preference rather than a clear error. |
| 1766 | [Nerdist](https://nerdist.com/) | Entertainment | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 1768 | [IGN](https://www.ign.com) | Entertainment | Consider Video Games | Gaming-heavy coverage (three of five samples directly concern games or game commerce/security), alongside anime and films. Entertainment remains defensible for the broad feed. |
| 8965 | [BBC Sport — F1](https://feeds.bbci.co.uk/sport/formula1/rss.xml) | F1 News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 8966 | [Autosport — F1](https://www.autosport.com/rss/f1/news/) | F1 News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 8967 | [Motorsport.com — F1](https://www.motorsport.com/rss/f1/news/) | F1 News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 8968 | [The Guardian — F1](https://www.theguardian.com/sport/formulaone/rss) | F1 News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 7093 | [NPR](https://feeds.npr.org/1001/rss.xml) | General News | Keep | Mixed news and service journalism fits General News. Moving to Top Headlines is optional policy, not a correction. |
| 8904 | [CBS News](https://www.cbsnews.com/latest/rss/main) | General News | Keep | Broad latest news feed includes 48 Hours promos and personal finance; General News is reasonable. |
| 9401 | [St. Louis Post-Dispatch](https://www.stltoday.com/search/?f=rss&t=article&l=50&s=start_time&sd=desc) | General News | Keep | General News fits the mixed local/national/sports/opinion feed. Not a dedicated St. Louis local feed. |
| 9172 | [KSHB 41](https://www.kshb.com/news/local-news.rss) | Kansas City News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9173 | [Kansas City PBS](https://flatlandkc.org/feed/) | Kansas City News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9174 | [The Pitch](https://www.thepitchkc.com/category/food-drink/feed/) | Kansas City News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9175 | [KCUR](https://www.kcur.org/tags/kansas-city.rss) | Kansas City News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9176 | [The Beacon — Kansas City](https://thebeaconnews.org/stories/category/kansas-city/feed/) | Kansas City News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 16 | [Linux Today](https://www.linuxtoday.com/) | Linux | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 17 | [9to5Linux](https://9to5linux.com/) | Linux | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 18 | [Phoronix](https://www.phoronix.com/) | Linux | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 20 | [It's FOSS](https://itsfoss.com/) | Linux | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 21 | [Linuxiac](https://linuxiac.com/) | Linux | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9353 | [ESPN — Men’s College Basketball](https://www.espn.com/espn/rss/ncb/news) | Men's College Basketball News | Keep; occasional spillover | Correct college basketball endpoint. One stored sample used an NHL fan reaction as a cross-sport comparison; live feed is basketball-focused. |
| 9354 | [CBS Sports — Men’s College Basketball](https://www.cbssports.com/rss/headlines/college-basketball/) | Men's College Basketball News | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9356 | [Heartland College Sports — Big 12 Basketball](https://www.heartlandcollegesports.com/category/big-12-basketball/feed/) | Men's College Basketball News | Keep; monitor scope | Current samples fit men’s basketball. URL is Big 12 basketball without an explicit men-only qualifier, so a future women’s story would be a feed-scope issue. |
| 297 | [ESPN](https://www.espn.com/espn/rss/news) | Sports | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 299 | [CBS Sports](https://www.cbssports.com/rss/headlines/) | Sports | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 708 | [Redbird Rants](https://redbirdrants.com/) | Sports | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 2 | [Engadget](https://www.engadget.com/) | Technology | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 22 | [XDA Developers](https://www.xda-developers.com/) | Technology | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 365 | [How-To Geek](https://www.howtogeek.com/) | Technology | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 367 | [Android Authority](https://www.androidauthority.com/) | Technology | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 709 | [TechRadar](https://www.techradar.com/) | Technology | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 3792 | [Windows Central](https://www.windowscentral.com/) | Technology | Keep; consider narrower feed | Windows/platform coverage fits Technology, but three of five stored samples concern gaming. Narrowing the feed would improve separation. |
| 4085 | [Android Police](https://www.androidpolice.com/) | Technology | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 4086 | [Droid Life](https://www.droid-life.com/) | Technology | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 7090 | [TheVerge](https://www.theverge.com/rss/index.xml) | Technology | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 7091 | [ArsTechnica](https://feeds.arstechnica.com/arstechnica/index) | Technology | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 4084 | [BBC News](https://feeds.bbci.co.uk/news/rss.xml) | Top Headlines | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 7092 | [PBS Newshour](https://www.pbs.org/newshour/feeds/rss/headlines) | Top Headlines | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 8903 | [NBC News](https://feeds.nbcnews.com/nbcnews/public/news) | Top Headlines | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 8905 | [The New York Times](https://rss.nytimes.com/services/xml/rss/nyt/HomePage.xml) | Top Headlines | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 8906 | [The Guardian](https://www.theguardian.com/international/rss) | Top Headlines | Review feed scope | International RSS includes sport, arts, reviews and features. Three of five stored samples were sport/music; live feed also mixed opinion and entertainment. A broad latest feed does not guarantee top headlines. |
| 8907 | [Al Jazeera](https://www.aljazeera.com/xml/rss/all.xml) | Top Headlines | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 8908 | [Sky News](https://feeds.skynews.com/feeds/rss/home.xml) | Top Headlines | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 300 | [Deadline](https://deadline.com/) | TV/Film | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 301 | [TVLine](https://www.tvline.com/) | TV/Film | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 366 | [/Film](https://www.slashfilm.com/) | TV/Film | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 1765 | [THR](https://www.hollywoodreporter.com/) | TV/Film | Keep; consider narrower feed | TV/Film is reasonable for the publisher, but fashion and Spotify earnings appear in the sample. A film/TV feed could reduce spillover. |
| 7895 | [Awful Announcing](https://awfulannouncing.com/) | TV/Film | Move to Sports | Sports broadcasting, announcers, ratings and viewing schedules; all five sampled stories fit sports media. |
| 19 | [GamingOnLinux](https://www.gamingonlinux.com/) | Video Games | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 1764 | [Pure Xbox](https://www.purexbox.com/) | Video Games | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 4088 | [Eurogamer](https://www.eurogamer.net/) | Video Games | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9037 | [Indie Games Plus](https://indiegamesplus.com/feed/) | Video Games | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9038 | [The Indie Informer](https://the-indie-in-former.com/feed/) | Video Games | Keep | Feed scope and five recent stored samples fit the assigned category. |
| 9039 | [Indie Hive](https://indie-hive.com/feed/) | Video Games | Keep | Feed scope and five recent stored samples fit the assigned category. |

## Verification limits and documentation drift

Live HTTP/XML spot checks succeeded for Guardian international RSS, ESPN college basketball, Polygon RSS, Windows Central RSS and CBS News RSS. Heartland basketball reset the connection on this check; its current stored source health was clear. A guessed IGN RSS path returned 404; the configured source is its homepage, so this does not establish an ingestion failure. No full collector run was performed.

Live categories/labels supersede older handoff entries: category 2627 is now General News; CBS News 8904 is there, BBC 4084 and PBS 7092 are in Top Headlines. ZAGSBLOG 9355, listed in the older handoff, is absent from the current Sources inventory. Its removal reason was not established. STLtoday’s current label is St. Louis Post-Dispatch. Do not restore old assignments or re-add absent sources solely from historical notes.

The application assigns categories at source level. Reassigning a source will also change the bucket of its existing articles wherever the frontend joins through Sources. Preserve article IDs/ownership and review frontend expectations before changing taxonomy.
