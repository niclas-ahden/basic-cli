app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.Locale

# Getting the preferred locale and all available locales

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args| {
    locale_str = match Locale.get!() {
        Ok(locale) => locale
        Err(NotAvailable) => "<not available>"
    }
    Stdout.line!("The most preferred locale for this system or application: ${locale_str}") ? |_| Exit(1)

    all_locales = Locale.all!()
    locales_str = Str.join_with(all_locales, ", ")
    Stdout.line!("All available locales for this system or application: [${locales_str}]") ? |_| Exit(1)

    Ok({})
}
