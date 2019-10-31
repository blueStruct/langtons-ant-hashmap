use self::Color::*;
use self::Dir::*;
use std::borrow::Borrow;

#[cfg(feature = "btree_map")]
use std::collections::btree_map::{BTreeMap as Map, Entry::*};

#[cfg(not(feature = "btree_map"))]
use fnv::FnvHashMap as Map;
#[cfg(not(feature = "btree_map"))]
use std::collections::hash_map::Entry::*;

use std::ops::Deref;
use std::ops::{Index, IndexMut};
use std::rc::Rc;
use std::time::Instant;

fn main() {
    let n: f32 = std::env::args()
        .nth(1)
        .expect("need number of turns")
        .parse()
        .expect("not a number");

    let mut ant = Ant::new();

    let now = Instant::now();
    for _ in 0..n as usize {
        ant.do_one_turn();
    }
    let elapsed = now.elapsed();

    println!("this took {} seconds", elapsed.as_secs());
    println!("registry used: {}", ant.registry.keys().count(),);
    println!("map used: {}", ant.map.keys().count(),);
    println!("black tiles: {}", ant.count_black_tiles());
}

const GRID_SIZE: usize = 32;
#[cfg(not(feature = "btree_map"))]
const REGISTRY_CAPACITY: usize = 512;
#[cfg(not(feature = "btree_map"))]
const MAP_CAPACITY: usize = 2e5 as usize;
const LAST_ELEM: usize = last_elem(GRID_SIZE);
const fn last_elem(a: usize) -> usize {
    a - 1
}

type CGrid = Grid<Color>;

#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord)]
struct Grid<T> {
    h: usize,
    w: usize,
    inner: Vec<T>,
}

struct Ant {
    registry: Map<CGrid, Rc<CGrid>>,
    map: Map<(i64, i64), Rc<CGrid>>,
    gx: i64,
    gy: i64,
    grid: CGrid,
    lx: usize,
    ly: usize,
    dir: Dir,
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug, PartialOrd, Ord)]
enum Color {
    White,
    Black,
}

#[derive(Copy, Clone)]
enum Dir {
    Up,
    Right,
    Down,
    Left,
}

impl Ant {
    fn new() -> Self {
        let empty_grid_rc = Rc::new(CGrid::new(GRID_SIZE, GRID_SIZE));

        #[cfg(feature = "btree_map")]
        let mut registry = Map::new();
        #[cfg(feature = "btree_map")]
        let mut map = Map::new();

        #[cfg(not(feature = "btree_map"))]
        let mut registry = Map::with_capacity_and_hasher(REGISTRY_CAPACITY, Default::default());
        #[cfg(not(feature = "btree_map"))]
        let mut map = Map::with_capacity_and_hasher(MAP_CAPACITY, Default::default());

        registry.insert(CGrid::new(GRID_SIZE, GRID_SIZE), empty_grid_rc.clone());
        map.insert((0, 0), empty_grid_rc);

        Self {
            registry,
            map,
            grid: CGrid::new(GRID_SIZE, GRID_SIZE),
            dir: Up,
            gx: 0,
            gy: 0,
            lx: GRID_SIZE / 2,
            ly: GRID_SIZE / 2,
        }
    }

    fn go_to_new_grid(&mut self, new_gx: i64, new_gy: i64) {
        // store old grid
        // if already in registry, fetch rc, store clone in map
        if let Some(rc) = self.registry.get(&self.grid) {
            self.map.insert((self.gx, self.gy), rc.clone());
        // else make new rc and store in registry and map
        } else {
            let rc = Rc::new(self.grid.clone());
            self.registry.insert(self.grid.clone(), rc.clone());
            self.map.insert((self.gx, self.gy), rc);
        }
        // load new grid
        self.grid = {
            let entry = self.map.entry((new_gx, new_gy));
            match entry {
                // if already existing, get it
                Occupied(x) => x.get().deref().borrow().clone(),
                // else return empty grid
                Vacant(_) => CGrid::new(GRID_SIZE, GRID_SIZE),
            }
        };
    }

    fn do_one_turn(&mut self) {
        let tile = &mut self.grid[(self.ly, self.lx)];

        // turn and flip color
        match tile {
            White => {
                self.dir = Dir::from(self.dir as i8 + 1);
                *tile = Black;
            }
            Black => {
                self.dir = Dir::from(self.dir as i8 - 1);
                *tile = White;
            }
        }

        // move forward, possibly to new grid
        match (self.dir, self.lx, self.ly) {
            (Up, _, 0) => {
                self.go_to_new_grid(self.gx, self.gy - 1);
                self.gy -= 1;
                self.ly = LAST_ELEM;
            }
            (Up, _, _) => {
                self.ly -= 1;
            }
            (Down, _, LAST_ELEM) => {
                self.go_to_new_grid(self.gx, self.gy + 1);
                self.gy += 1;
                self.ly = 0;
            }
            (Down, _, _) => {
                self.ly += 1;
            }
            (Left, 0, _) => {
                self.go_to_new_grid(self.gx - 1, self.gy);
                self.gx -= 1;
                self.lx = LAST_ELEM;
            }
            (Left, _, _) => {
                self.lx -= 1;
            }
            (Right, LAST_ELEM, _) => {
                self.go_to_new_grid(self.gx + 1, self.gy);
                self.gx += 1;
                self.lx = 0;
            }
            (Right, _, _) => {
                self.lx += 1;
            }
        }
    }

    fn count_black_tiles(&mut self) -> usize {
        let mut n = 0;

        // update current grid in HashMap
        self.map
            .insert((self.gx, self.gy), Rc::new(self.grid.clone()));

        // count tiles
        for (_, rc_grid) in self.map.iter() {
            n += rc_grid.iter().filter(|&&x| x == Black).count();
        }

        n
    }
}

impl<T> Grid<T>
where
    T: Default + Clone,
{
    fn new(h: usize, w: usize) -> Self {
        Self {
            h,
            w,
            inner: vec![T::default(); h * w],
        }
    }

    fn iter(&self) -> std::slice::Iter<T> {
        self.inner.iter()
    }
}

impl<T> Index<(usize, usize)> for Grid<T> {
    type Output = T;

    fn index(&self, (y, x): (usize, usize)) -> &Self::Output {
        &self.inner[y * self.w + x]
    }
}

impl<T> IndexMut<(usize, usize)> for Grid<T> {
    fn index_mut(&mut self, (y, x): (usize, usize)) -> &mut Self::Output {
        &mut self.inner[y * self.w + x]
    }
}

impl Default for Color {
    fn default() -> Self {
        White
    }
}

impl From<i8> for Dir {
    fn from(a: i8) -> Self {
        let a = a % 4;
        match a {
            0 => Up,
            1 => Right,
            2 => Down,
            3 => Left,
            _ => Up,
        }
    }
}

impl From<Dir> for i8 {
    fn from(a: Dir) -> Self {
        match a {
            Up => 0,
            Right => 1,
            Down => 2,
            Left => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_turn() {
        let mut ant = Ant::new();
        ant.do_one_turn();
        assert_eq!(ant.count_black_tiles(), 1);
    }

    #[test]
    fn four_turns() {
        let mut ant = Ant::new();
        for _ in 0..4 {
            ant.do_one_turn();
        }
        assert_eq!(ant.count_black_tiles(), 4);
    }

    #[test]
    fn five_turns() {
        let mut ant = Ant::new();
        for _ in 0..5 {
            ant.do_one_turn();
        }
        assert_eq!(ant.count_black_tiles(), 3);
    }

    #[test]
    fn a_lot_of_turns() {
        let mut ant = Ant::new();
        for _ in 0..10000 {
            ant.do_one_turn();
        }
    }
}
