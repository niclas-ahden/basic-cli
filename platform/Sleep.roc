import Host

## Pause the current process for a requested duration.
Sleep :: [].{

	## Sleep for the specified number of milliseconds.
	millis! : U64 => {}
	millis! = |milliseconds| Host.sleep_millis!(milliseconds)
}
