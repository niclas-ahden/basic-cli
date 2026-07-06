import Host

Sleep := [].{
    ## Sleep for the specified number of milliseconds.
    millis! : U64 => {}
    millis! = |milliseconds| Host.sleep_millis!(milliseconds)
}
