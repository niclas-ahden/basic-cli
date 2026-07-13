app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Url

# Build a URL for a search request without hand-encoding user input.

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	base = Url.from_str("https://api.example.com")
	search = Url.append(base, "v1/search")
	with_query = Url.append_param(search, "q", "roc lang")
	with_page = Url.append_param(with_query, "page", "1")
	url = Url.with_fragment(with_page, "results")

	expect Url.path(url) == "v1/search"
	expect Url.query(url) == "q=roc%20lang&page=1"

	Stdout.line!("Request URL: ${Url.to_str(url)}")?
	Ok({})
}
