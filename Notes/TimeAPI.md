
Linked by eternaleye on #rust-osdev

[https://github.com/rust-lang/rfcs/pull/1288#discussion_r40244672](rust-lang/rfcs#1288 Comment by @eternaleye)

> There's also that there are multiple kinds of monotonic time on some systems.
>
> In particular, while `CLOCK_MONOTONIC` generates comparable and orderable instants, it does not generate comparable durations because it is still affected by NTP's ability to alter the length of a second. Linux at least has `CLOCK_MONOTONIC_RAW` (whose ticks are of constant length), but it makes no guarantee of the relationship between its units and human-meaningful ones.
>
> In addition, suspend/hibernate is not counted for `CLOCK_MONOTONIC`, so durations from it are even more suspect - but Linux has the (underdocumented) `CLOCK_BOOTTIME`, which does count that... however, it isn't clear what its guarantees on the length of a tick are (consistent vs. meaningful).
> 
> Yes, this is in fact worth curling up in a corner over. No, it likely doesn't get better.
> 
> EDIT:
> 
> Personally, what I wish the platform provided was something akin to three calls:
> 
> booted() -> TAI nanoseconds
> offset(kind) -> Consistent-length ticks since booted()
> scale() -> Ratio ns/ticks
> 
> where 'kind' is one of "run" or "boot", respectively meaning actual runtime (`CLOCK_MONOTONIC_RAW`) or external time since boot (a hypothetical `CLOCK_BOOTTIME_RAW`) respectively.
> 
> The workflow would then be:
>
> ``` 
>  // take measurements
> let start = offset(kind);
> do_thing();
> let end = offset(kind);
> // Get a comparable duration
> let duration = end - start;
> // Get _coherent_ view of what that means in real units
> let ratio = scale();
> println!("do_thing() took {} nanoseconds", duration * ratio);
> // Get the beginning of time, to format as human-readable instants
> let base = booted();
> println!(
>     "It began at {:date} and finished at {:date}",
>     base + start * ratio; base + end * ratio
> );
> ```
> 
> One can at least approximate those on Linux systems in some cases:
> 
>     booted(): CLOCK_TAI - CLOCK_MONOTONIC
>     offset(run): CLOCK_MONOTONIC_RAW
>     scale(): possibly CLOCK_MONOTONIC / CLOCK_MONOTONIC_RAW, but that relies on NTP's second-slewing averaging out to a second, which is likely optimistic when it also uses it to correct small initial errors. It also doesn't work for BOOTTIME because there is no _RAW variant. Eugh. :/
> 

