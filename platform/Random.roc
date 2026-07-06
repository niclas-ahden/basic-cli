import IOErr exposing [IOErr]
import Host

Random := [].{
    ## Generate a random 64-bit unsigned integer seed.
    seed_u64! : () => Try(U64, [RandomErr(IOErr), ..])
    seed_u64! = || Ok(Host.random_seed_u64!()?)

    ## Generate a random 32-bit unsigned integer seed.
    seed_u32! : () => Try(U32, [RandomErr(IOErr), ..])
    seed_u32! = || Ok(Host.random_seed_u32!()?)
}
