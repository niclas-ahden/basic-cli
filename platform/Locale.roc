import Host

Locale := [].{
    ## Returns the most preferred locale for the system or application.
    ##
    ## The returned `Str` is a BCP 47 language tag, like `en-US` or `fr-CA`.
    ##
    ## Returns `Err(NotAvailable)` if the locale cannot be determined.
    get! : () => Try(Str, [NotAvailable, ..])
    get! = || widen_locale_err(Host.locale_get!())

    ## Returns the preferred locales for the system or application.
    ##
    ## The returned `Str`s are BCP 47 language tags, like `en-US` or `fr-CA`.
    all! : () => List(Str)
    all! = || Host.locale_all!()
}

widen_locale_err : Try(a, [NotAvailable]) -> Try(a, [NotAvailable, ..])
widen_locale_err = |result|
    match result {
        Ok(value) => Ok(value),
        Err(NotAvailable) => Err(NotAvailable),
    }
