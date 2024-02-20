# Voyager

Voyager is the future server back-end for [Endless Void](https://github.com/Skirlez/void-stranger-endless-void), a level editor for [Void Stranger](https://store.steampowered.com/app/2121980/Void_Stranger/), a "2D sokoban-style puzzle game where every step counts."

## The philosophy going forward

In terms of priorities for this project, generally, `idiomatic > small > fast`. Previously, Voyager's priorities have been something like `fast > small > idiomatic`, for some reason. Despite having a cool name, Voyager is a very simple project. At its core, it's a glorified key-value database (and its internal data structure currently reflects that), with its most complex feature being (ideally) a very simple level parsing/validation (essentially, "does this kind of look somewhat like how a level should?"). 

### Idiomatic

Idiomatic code is good not only for readability, but also for practice and for showing off. Since there's no real need to optimize for size nor for performance, why not write the most beautiful code possible?

### Small

Mostly referring to memory usage, it would generally be a good idea to decrease it, especially since the free VPS from Oracle where Voyager will be running only has 1GB of RAM. Thankfully, the average Void Stranger level is incredibly tiny (<1KB), and simply storing them and their key in a HashMap (or, currently, a DashMap) is incredibly difficult to mess up.

### Fast

Speed is a feature. Despite this, an end-user is unlikely to notice their level uploading in 400µs instead of 500µs. Rust and Axum are plenty fast enough, and Voyager is unlikely to have more than 2 concurrent users. Therefore, optimizing for speed, and the concept of speed in general, should be an afterthought, as striving for speed generally leads to premature optimization (speaking from experience).

## To-do list

- [ ] PUT router.
- [ ] Improved logging.
- [ ] Comprehensive testing.
- [ ] Extensive documenting.
- [ ] Appropriate simplifying.
