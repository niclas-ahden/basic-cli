app [main!] { pf: platform "../platform/main.roc" }

# Demo of basic-cli Random functions

import pf.Stdout
import pf.Random

main! : List(Str) => Try({}, _)
main! = |_args| {
    random_u64 = Random.seed_u64!()?
    Stdout.line!("Random U64 seed is: ${random_u64.to_str()}")?

    random_u32 = Random.seed_u32!()?
    Stdout.line!("Random U32 seed is: ${random_u32.to_str()}")?

    Ok({})
}
