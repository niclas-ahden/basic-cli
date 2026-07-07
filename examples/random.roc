app [main!] { pf: platform "../platform/main.roc" }

# Demo of basic-cli Random functions

import pf.Stdout
import pf.Random

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args| {
    result_u64 = Random.seed_u64!()
    match result_u64 {
        Ok(random_u64) => {
            Stdout.line!("Random U64 seed is: ${random_u64.to_str()}") ? |_| Exit(1)
            result_u32 = Random.seed_u32!()
            match result_u32 {
                Ok(random_u32) => {
                    Stdout.line!("Random U32 seed is: ${random_u32.to_str()}") ? |_| Exit(1)
                    Ok({})
                }
                Err(_) => {
                    Stdout.line!("Failed to generate random U32 seed") ? |_| Exit(1)
                    Err(Exit(1))
                }
            }
        }
        Err(_) => {
            Stdout.line!("Failed to generate random U64 seed") ? |_| Exit(1)
            Err(Exit(1))
        }
    }
}
