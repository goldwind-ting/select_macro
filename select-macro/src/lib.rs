extern crate select_macro_utils;


pub fn thread_rng_n(branches: u32) -> u32{
    fastrand::u32(0..branches)
}

#[macro_export]
macro_rules! select {
    (@ {
        start=$start:expr;
        ( $($count:tt)* )
        $( ( $($skip:tt)* ) $bind:pat = $fut:expr, if $c:expr => $handle:expr, )+

        ; $else:expr

    }) => {{

        #[doc(hidden)]
        mod __select_util {
            select_macro_utils::select_priv_declare_output_enum!( ( $($count)* ) );
        }


        const BRANCHES: u32 = count!( $($count)* );

        let mut disabled: __select_util::Mask = Default::default();

        $(
            if !$c {
                let mask: __select_util::Mask = 1 << count!( $($skip)* );
                disabled |= mask;
            }
        )*

        let output = {
            let mut futures = ( $( $fut , )+ );

            let futures = &mut futures;

            futures::future::poll_fn(|cx| {
                let mut is_pending = false;
                let start = $start;

                for i in 0..BRANCHES {
                    let branch;
                    #[allow(clippy::modulo_one)]
                    {
                        branch = (start + i) % BRANCHES;
                    }
                    match branch {
                        $(
                            #[allow(unreachable_code)]
                            count!( $($skip)* ) => {
                                let mask = 1 << branch;

                                if disabled & mask == mask {
                                    continue;
                                }


                                let ( $($skip,)* fut, .. ) = &mut *futures;

                                let fut = unsafe { std::pin::Pin::new_unchecked(fut) };

                                let out = match std::future::Future::poll(fut, cx) {
                                    std::task::Poll::Ready(out) => out,
                                    std::task::Poll::Pending => {
                                        is_pending = true;
                                        continue;
                                    }
                                };

                                disabled |= mask;

                                #[allow(unused_variables)]
                                #[allow(unused_mut)]
                                match &out {
                                    select_macro_utils::select_priv_clean_pattern!($bind) => {},
                                    _ => continue,
                                }

                                return std::task::Poll::Ready(select_variant!(__select_util::Out, ($($skip)*))(out));
                            }
                        )*
                        _ => unreachable!("reaching this means there probably is an off by one bug"),
                    }
                }

                if is_pending {
                    std::task::Poll::Pending
                } else {
                    std::task::Poll::Ready(__select_util::Out::Disabled)
                }
            }).await
        };

        match output {
            $(
                select_variant!(__select_util::Out, ($($skip)*) ($bind)) => $handle,
            )*
            __select_util::Out::Disabled => $else,
            _ => unreachable!("failed to match bind"),
        }
    }};

    // ==== Normalize =====

    // These rules match a single `select!` branch and normalize it for
    // processing by the first rule.

    (@ { start=$start:expr; $($t:tt)* } ) => {
        // No `else` branch
        $crate::select!(@{ start=$start; $($t)*; panic!("all branches are disabled and there is no else branch") })
    };
    (@ { start=$start:expr; $($t:tt)* } else => $else:expr $(,)?) => {
        $crate::select!(@{ start=$start; $($t)*; $else })
    };
    (@ { start=$start:expr; ( $($s:tt)* ) $($t:tt)* } $p:pat = $f:expr, if $c:expr => $h:block, $($r:tt)* ) => {
        $crate::select!(@{ start=$start; ($($s)* _) $($t)* ($($s)*) $p = $f, if $c => $h, } $($r)*)
    };
    (@ { start=$start:expr; ( $($s:tt)* ) $($t:tt)* } $p:pat = $f:expr => $h:block, $($r:tt)* ) => {
        $crate::select!(@{ start=$start; ($($s)* _) $($t)* ($($s)*) $p = $f, if true => $h, } $($r)*)
    };
    (@ { start=$start:expr; ( $($s:tt)* ) $($t:tt)* } $p:pat = $f:expr, if $c:expr => $h:block $($r:tt)* ) => {
        $crate::select!(@{ start=$start; ($($s)* _) $($t)* ($($s)*) $p = $f, if $c => $h, } $($r)*)
    };
    (@ { start=$start:expr; ( $($s:tt)* ) $($t:tt)* } $p:pat = $f:expr => $h:block $($r:tt)* ) => {
        $crate::select!(@{ start=$start; ($($s)* _) $($t)* ($($s)*) $p = $f, if true => $h, } $($r)*)
    };
    (@ { start=$start:expr; ( $($s:tt)* ) $($t:tt)* } $p:pat = $f:expr, if $c:expr => $h:expr ) => {
        $crate::select!(@{ start=$start; ($($s)* _) $($t)* ($($s)*) $p = $f, if $c => $h, })
    };
    (@ { start=$start:expr; ( $($s:tt)* ) $($t:tt)* } $p:pat = $f:expr => $h:expr ) => {
        $crate::select!(@{ start=$start; ($($s)* _) $($t)* ($($s)*) $p = $f, if true => $h, })
    };
    (@ { start=$start:expr; ( $($s:tt)* ) $($t:tt)* } $p:pat = $f:expr, if $c:expr => $h:expr, $($r:tt)* ) => {
        $crate::select!(@{ start=$start; ($($s)* _) $($t)* ($($s)*) $p = $f, if $c => $h, } $($r)*)
    };
    (@ { start=$start:expr; ( $($s:tt)* ) $($t:tt)* } $p:pat = $f:expr => $h:expr, $($r:tt)* ) => {
        $crate::select!(@{ start=$start; ($($s)* _) $($t)* ($($s)*) $p = $f, if true => $h, } $($r)*)
    };

    // ===== Entry point =====

    (biased; $p:pat = $($t:tt)* ) => {
        $crate::select!(@{ start=0; () } $p = $($t)*)
    };

    ( $p:pat = $($t:tt)* ) => {
        // Randomly generate a starting point. This makes `select!` a bit more
        // fair and avoids always polling the first future.
        $crate::select!(@{ start={ $crate::thread_rng_n(BRANCHES) }; () } $p = $($t)*)
    };
    () => {
        compile_error!("select! requires at least one branch.")
    };
}

#[macro_export]
macro_rules! count {
    () => {
        0
    };
    (_) => {
        1
    };
    (_ _) => {
        2
    };
    (_ _ _) => {
        3
    };
    (_ _ _ _) => {
        4
    };
    (_ _ _ _ _) => {
        5
    };
    (_ _ _ _ _ _) => {
        6
    };
    (_ _ _ _ _ _ _) => {
        7
    };
    (_ _ _ _ _ _ _ _) => {
        8
    };
    (_ _ _ _ _ _ _ _ _) => {
        9
    };
    (_ _ _ _ _ _ _ _ _ _) => {
        10
    };
    (_ _ _ _ _ _ _ _ _ _ _) => {
        11
    };
    (_ _ _ _ _ _ _ _ _ _ _ _) => {
        12
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _) => {
        13
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        14
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        15
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        16
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        17
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        18
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        19
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        20
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        21
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        22
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        23
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        24
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        25
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        26
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        27
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        28
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        29
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        30
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        31
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        32
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        33
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        34
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        35
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        36
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        37
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        38
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        39
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        40
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        41
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        42
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        43
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        44
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        45
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        46
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        47
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        48
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        49
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        50
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        51
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        52
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        53
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        54
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        55
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        56
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        57
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        58
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        59
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        60
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        61
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        62
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        63
    };
    (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) => {
        64
    };
}

#[macro_export]
macro_rules! select_variant {
    ($($p:ident)::*, () $($t:tt)*) => {
        $($p)::*::_0 $($t)*
    };
    ($($p:ident)::*, (_) $($t:tt)*) => {
        $($p)::*::_1 $($t)*
    };
    ($($p:ident)::*, (_ _) $($t:tt)*) => {
        $($p)::*::_2 $($t)*
    };
    ($($p:ident)::*, (_ _ _) $($t:tt)*) => {
        $($p)::*::_3 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _) $($t:tt)*) => {
        $($p)::*::_4 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_5 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_6 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_7 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_8 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_9 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_10 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_11 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_12 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_13 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_14 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_15 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_16 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_17 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_18 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_19 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_20 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_21 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_22 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_23 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_24 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_25 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_26 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_27 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_28 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_29 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_30 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_31 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_32 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_33 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_34 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_35 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_36 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_37 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_38 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_39 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_40 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_41 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_42 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_43 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_44 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_45 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_46 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_47 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_48 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_49 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_50 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_51 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_52 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_53 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_54 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_55 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_56 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_57 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_58 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_59 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_60 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_61 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_62 $($t)*
    };
    ($($p:ident)::*, (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _) $($t:tt)*) => {
        $($p)::*::_63 $($t)*
    };
}
