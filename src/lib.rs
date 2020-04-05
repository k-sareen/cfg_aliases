/// Parse cfg aliases and output cargo cfg aliases
///
/// As an example:
///
/// ```rust
/// # use cfg_aliases::cfg_aliases
///
/// // Setup cfg aliases
/// cfg_aliases! {
///     // Platforms
///     wasm: { target_arch = "wasm32" },
///     android: { target_os = "android" },
///     macos: { target_os = "macos" },
///     linux: { target_os = "linux" },
///     // Backends
///     surfman: { all(unix, feature = "surfman", not(wasm)) },
///     glutin: { all(feature = "glutin", not(wasm)) },
///     wgl: { all(windows, feature = "wgl", not(wasm)) },
///     dummy: { not(any(wasm, glutin, wgl, surfman)) },
/// }
/// ```
///
/// After you put this in your build script you can then check for those conditions like so:
///
/// ```rust
/// #[cfg(surfman)]
/// {
///     // Do stuff related to surfman
/// }
///
/// #[cfg(dummy)]
/// println!("We're in dummy mode, specify another feature if you want a smarter app!");
/// ```
///
/// This greatly improves what would otherwise look like this without the aliases:
///
/// ```rust
/// #[cfg(all(unix, feature = "surfman", not(target_arch = "wasm32"))))]
/// {
///     // Do stuff related to surfman
/// }
///
/// #[cfg(not(any(
///     target_arch = "wasm32",
///     all(unix, feature = "surfman", not(target_arch = "wasm32")),
///     all(windows, feature = "wgl", not(target_arch = "wasm32")),
///     all(feature = "glutin", not(target_arch = "wasm32")),
/// )))]
/// println!("We're in dummy mode, specify another feature if you want a smarter app!");
/// ```
///
/// ### Thanks
///
/// The majority of the `cfg` syntax tt muncher pattern for this macro was taken from the
/// [`tectonic_cfg_support::target_cfg`] macro. Also thanks to @ratmice for [bringing it up][bip]
/// on the Rust forum.
///
/// Thanks to @Yandros on the Rust forum as well for [showing me][sm] some crazy macro hacks!
///
/// And thanks to my God and Father who led me to this and to whome I owe everything.
///
/// [`tectonic_cfg_support::target_cfg`]: https://docs.rs/tectonic_cfg_support/0.0.1/src/tectonic_cfg_support/lib.rs.html#166-298
/// [bip]: https://users.rust-lang.org/t/any-such-thing-as-cfg-aliases/40100/13
/// [sm]: https://users.rust-lang.org/t/any-such-thing-as-cfg-aliases/40100/3
#[macro_export]
macro_rules! cfg_aliases {
    // Helper that just checks whether the CFG environment variable is set
    (@cfg_is_set $cfgname:ident) => {
        std::env::var(
            format!(
                "CARGO_CFG_{}",
                &stringify!($cfgname).to_uppercase().replace("-", "_")
            )
        ).is_ok()
    };
    // Helper to check for the presense of a feature
    (@cfg_has_feature $feature:expr) => {
        {
            std::env::var(
                format!(
                    "CARGO_FEATURE_{}",
                    &stringify!($feature).to_uppercase().replace("-", "_").replace('"', "")
                )
            ).map(|x| x == "1").unwrap_or(false)
        }
    };

    // Helper that checks whether a CFG environment contains the given value
    (@cfg_contains $cfgname:ident = $cfgvalue:expr) => {
        std::env::var(
            format!(
                "CARGO_CFG_{}",
                &stringify!($cfgname).to_uppercase().replace("-", "_")
            )
        ).unwrap_or("".to_string()).split(",").find(|x| x == &$cfgvalue).is_some()
    };

    // Emitting `any(clause1,clause2,...)`: convert to `$crate::cfg_aliases!(clause1) && $crate::cfg_aliases!(clause2) && ...`
    (
        @parser_emit
        all
        $({$($grouped:tt)+})+
    ) => {
        ($(
            ($crate::cfg_aliases!(@parser $($grouped)+))
        )&&+)
    };

    // Likewise for `all(clause1,clause2,...)`.
    (
        @parser_emit
        any
        $({$($grouped:tt)+})+
    ) => {
        ($(
            ($crate::cfg_aliases!(@parser $($grouped)+))
        )||+)
    };

    // "@clause" rules are used to parse the comma-separated lists. They munch
    // their inputs token-by-token and finally invoke an "@emit" rule when the
    // list is all grouped. The general pattern for recording the parser state
    // is:
    //
    // ```
    // $crate::cfg_aliases!(
    //    @clause $operation
    //    [{grouped-clause-1} {grouped-clause-2...}]
    //    [not-yet-parsed-tokens...]
    //    current-clause-tokens...
    // )
    // ```

    // This rule must come first in this section. It fires when the next token
    // to parse is a comma. When this happens, we take the tokens in the
    // current clause and add them to the list of grouped clauses, adding
    // delimeters so that the grouping can be easily extracted again in the
    // emission stage.
    (
        @parser_clause
        $op:ident
        [$({$($grouped:tt)+})*]
        [, $($rest:tt)*]
        $($current:tt)+
    ) => {
        $crate::cfg_aliases!(@parser_clause $op [
            $(
                {$($grouped)+}
            )*
            {$($current)+}
        ] [
            $($rest)*
        ]);
    };

    // This rule comes next. It fires when the next un-parsed token is *not* a
    // comma. In this case, we add that token to the list of tokens in the
    // current clause, then move on to the next one.
    (
        @parser_clause
        $op:ident
        [$({$($grouped:tt)+})*]
        [$tok:tt $($rest:tt)*]
        $($current:tt)*
    ) => {
        $crate::cfg_aliases!(@parser_clause $op [
            $(
                {$($grouped)+}
            )*
        ] [
            $($rest)*
        ] $($current)* $tok);
    };

    // This rule fires when there are no more tokens to parse in this list. We
    // finish off the "current" token group, then delegate to the emission
    // rule.
    (
        @parser_clause
        $op:ident
        [$({$($grouped:tt)+})*]
        []
        $($current:tt)+
    ) => {
        $crate::cfg_aliases!(@parser_emit $op
            $(
                {$($grouped)+}
            )*
            {$($current)+}
        );
    };


    // `all(clause1, clause2...)` : we must parse this comma-separated list and
    // partner with `@emit all` to output a bunch of && terms.
    (
        @parser
        all($($tokens:tt)+)
    ) => {
        $crate::cfg_aliases!(@parser_clause all [] [$($tokens)+])
    };

    // Likewise for `any(clause1, clause2...)`
    (
        @parser
        any($($tokens:tt)+)
    ) => {
        $crate::cfg_aliases!(@parser_clause any [] [$($tokens)+])
    };

    // `not(clause)`: compute the inner clause, then just negate it.
    (
        @parser
        not($($tokens:tt)+)
    ) => {
        !($crate::cfg_aliases!(@parser $($tokens)+))
    };

    // `feature = value`: test for a feature.
    (@parser feature = $value:expr) => {
        $crate::cfg_aliases!(@cfg_has_feature $value)
    };
    // `param = value`: test for equality.
    (@parser $key:ident = $value:expr) => {
        $crate::cfg_aliases!(@cfg_contains $key = $value)
    };
    // Parse a lone identifier that might be an alias
    (@parser $e:ident) => {
        __cfg_aliases_matcher__!($e)
    };

    // Entrypoint that defines the matcher
    (
        @with_dollar[$dol:tt]
        $( $alias:ident : { $($config:tt)* } ),* $(,)?
    ) => {
        // Create a macro that expands other aliases and outputs any non
        // alias by checking whether that CFG value is set
        macro_rules! __cfg_aliases_matcher__ {
            // Parse config expression for the alias
            $(
                ( $alias ) => {
                    $crate::cfg_aliases!(@parser $($config)*)
                };
            )*
            // Anything that doesn't match evaluate the item
            ( $dol e:ident ) => {
                $crate::cfg_aliases!(@cfg_is_set $dol e)
            };
        }

        $(
            if $crate::cfg_aliases!(@parser $($config)*) {
                println!("cargo:rustc-cfg={}", stringify!($alias));
            }
        )*
    };

    // Catch all that starts the macro
    ($($tokens:tt)*) => {
        $crate::cfg_aliases!(@with_dollar[$] $($tokens)*)
    }
}
