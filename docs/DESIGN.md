# Mourama design

Persistent 5-seat hillfort. Tribal Wars loop, Iberian names, faeOS chrome.

## Map

Axial hexes, radius 5. Five **seats** on ring 4. Remaining tiles are
**outeiros** (empty) or **mamoas** (wild mounds with a small garrison).

A claimed seat is a **citânia**. Further hills you bind stay **castros**.

## Resources

| Pile | Produced by | Spent on |
|---|---|---|
| Cobre | Mina | bronze (with estanho), some troops |
| Estanho | Veio | bronze (with cobre) |
| Seara | Eira | feeding / most training |
| Orvalho | Fonte | encanto, serpe, trasgo, falcão |

**Bronze** is not stored. Spending `N` bronze deducts `N` cobre and `N`
estanho together. Guerreiro de bronze and muralha want it.

## Buildings

Casa circular (HQ) · Eira · Mina · Veio · Fonte · Curral · Muralha · Anta · Celeiro.

A building's level cannot pass the casa. Upgrade timers are real unix time.
Production is per-second from the last accrue timestamp (catch-up on wake).

## Units

Pastor · Trasgo · Falcão · Javali · Guerreiro de bronze · Serpe.

Speed is seconds per hex from the slowest unit in the army. Falcão scouts
see true counts. Encanto veils the public map. A **serpe** plus an orvalho
offering binds a hill if the army also wins the fight.

## Five seats

The sixth account is refused. Seats are the five starting citânias.
One court may hold many hills.

## Clock

`Europe/Lisbon` via the machine's local zone (Portugal). Tick every 1s
while `mourama serve` runs; on restart, events and production catch up.
