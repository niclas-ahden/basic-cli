app [main!] { pf: platform "../platform/main.roc" }

import pf.OsStr exposing [OsStr]
import pf.Stdout
import pf.Locale

# Getting the preferred locale and all available locales

main! : List(OsStr) => Try({}, _)
main! = |_args| {
	locale_str = match Locale.get!() {
		Ok(locale) => locale.to_str()
		Err(NotAvailable) => "<not available>"
	}
	Stdout.line!("The most preferred locale for this system or application: ${locale_str}")?

	all_locales = Locale.all!()
	locales_str = Str.join_with(all_locales.map(|locale| locale.to_str()), ", ")
	Stdout.line!("All available locales for this system or application: [${locales_str}]")?

	Ok({})
}
