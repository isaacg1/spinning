use image::{ImageBuffer, RgbImage};
use noisy_float::prelude::*;
use rand::prelude::*;

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

type Color = [u8; 3];
type Location = [usize; 2];

#[derive(Debug, Clone, Copy)]
struct Pixel {
    color: Color,
    loc: Location,
    center: Location,
}

struct VecMap<T> {
    vec: Vec<T>,
    map: HashMap<T, usize>,
}
impl<T: Copy + Eq + Hash> VecMap<T> {
    fn new_from_vec(vec: Vec<T>) -> Self {
        let map = vec.iter().enumerate().map(|(i, &v)| (v, i)).collect();
        Self { vec, map }
    }
    fn remove_random<R: Rng>(&mut self, rng: &mut R) -> Option<T> {
        if self.vec.is_empty() {
            return None;
        }
        let index = rng.random_range(0..self.vec.len());
        let last = *self.vec.last().unwrap();
        let out = self.vec.swap_remove(index);
        self.map.remove(&out);
        if index != self.vec.len() {
            self.map.insert(last, index);
        }
        Some(out)
    }
    fn remove(&mut self, item: &T) -> bool {
        let maybe_index = self.map.remove(item);
        match maybe_index {
            Some(index) => {
                let last = *self.vec.last().unwrap();
                self.vec.swap_remove(index);
                if index != self.vec.len() {
                    self.map.insert(last, index);
                }
                true
            }
            None => false,
        }
    }
}

fn make_image(
    size: usize,
    num_centers: usize,
    num_lookback: usize,
    start_spread: f64,
    cont_spread: f64,
    seed: u64,
) -> RgbImage {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut grid: Vec<Vec<Option<Pixel>>> = vec![vec![None; size]; size];
    let mut lookback: VecDeque<Pixel> = VecDeque::new();
    let mut open_locs: VecMap<Location> = VecMap::new_from_vec(
        (0..size)
            .flat_map(|i| (0..size).map(move |j| [i, j]))
            .collect(),
    );
    for i in 0..size * size {
        let color = [rng.random(), rng.random(), rng.random()];
        let insert_random = &mut |open_locs: &mut VecMap<Location>,
                                  grid: &mut Vec<Vec<Option<Pixel>>>,
                                  lookback: &mut VecDeque<Pixel>| {
            let loc = open_locs.remove_random(&mut rng).expect("nonempty");
            //let center = [rng.random_range(0..size), rng.random_range(0..size)];
            let width = (size as f64 * start_spread) as usize;
            let center = [
                rng.random_range(loc[0].saturating_sub(width)..=(loc[0] + width).min(size - 1)),
                rng.random_range(loc[1].saturating_sub(width)..=(loc[1] + width).min(size - 1)),
            ];
            let pixel = Pixel { color, loc, center };
            grid[loc[0]][loc[1]] = Some(pixel);
            lookback.push_front(pixel);
            lookback.truncate(num_lookback);
        };
        if i < num_centers {
            insert_random(&mut open_locs, &mut grid, &mut lookback);
            continue;
        }
        let nearest = lookback
            .iter()
            .min_by_key(|pixel| {
                let pcolor = pixel.color;
                color
                    .iter()
                    .zip(pcolor)
                    .map(|(&c, pc)| (c as i64 - pc as i64).pow(2))
                    .sum::<i64>()
            })
            .expect("find one");
        // Walk around the circle until an open pixel is found,
        // or a boundary is encountered,
        // or reach start.
        let dist = &|loc: [isize; 2]| {
            loc.iter()
                .zip(nearest.center)
                .map(|(&l, cl)| (l as f64 - cl as f64).powi(2))
                .sum()
        };
        let start = [nearest.loc[0] as isize, nearest.loc[1] as isize];
        let mut last = start.clone();
        let mut cur = start.clone();
        let radius: f64 = dist(cur);
        let mut j = 0;
        loop {
            j += 1;
            let neighbors = [
                [cur[0] + 1, cur[1] + 1],
                [cur[0], cur[1] + 1],
                [cur[0] - 1, cur[1] + 1],
                [cur[0] + 1, cur[1]],
                [cur[0] - 1, cur[1]],
                [cur[0] + 1, cur[1] - 1],
                [cur[0], cur[1] - 1],
                [cur[0] - 1, cur[1] - 1],
            ];
            let next = neighbors
                .into_iter()
                .filter(|&n| n != last)
                .min_by_key(|&n| n64((dist(n) - radius).abs()))
                .expect("Still one left");
            if next == start
                || next[0] < 0
                || next[0] >= size as isize
                || next[1] < 0
                || next[1] >= size as isize
                || j as f64 > 8.0 * radius
            {
                insert_random(&mut open_locs, &mut grid, &mut lookback);
                break;
            }
            if grid[next[0] as usize][next[1] as usize].is_none() {
                let color_dist_sq = color
                    .iter()
                    .zip(nearest.color)
                    .map(|(&c, pc)| (c as i64 - pc as i64).pow(2))
                    .sum::<i64>();
                let width = (((color_dist_sq as f64).sqrt() * cont_spread) as usize).max(1);
                let center = //nearest.center;
                [
                    rng.random_range(
                        nearest.center[0].saturating_sub(width)
                            ..=(nearest.center[0] + width).min(size),
                    ),
                    rng.random_range(
                        nearest.center[1].saturating_sub(width)
                            ..=(nearest.center[1] + width).min(size),
                    ),
                ];
                let loc = [next[0] as usize, next[1] as usize];
                let pixel = Pixel { color, loc, center };
                /*
                if (pixel.loc[0] as isize - start[0]).abs()
                    == (pixel.loc[1] as isize - start[1]).abs()
                {
                    println!("{i} {j}\n{pixel:?}\n{nearest:?}");
                }
                */
                grid[loc[0]][loc[1]] = Some(pixel);
                open_locs.remove(&loc);
                lookback.push_front(pixel);
                lookback.truncate(num_lookback);
                break;
            }
            last = cur;
            cur = next;
        }
    }
    let mut img: RgbImage = ImageBuffer::new(size as u32, size as u32);
    for (i, row) in grid.into_iter().enumerate() {
        for (j, pixel) in row.into_iter().enumerate() {
            if let Some(pixel) = pixel {
                img.put_pixel(i as u32, j as u32, image::Rgb(pixel.color));
            }
        }
    }
    img
}

fn main() {
    let size = 1000;
    let num_centers = 20;
    let num_lookback = 1000;
    let start_spread = 0.5;
    let cont_spread = 0.1;
    let seed = 19;
    let filename =
        format!("img-{size}-{num_centers}-{num_lookback}-{start_spread}-{cont_spread}-{seed}.png");
    println!("Start {filename}");
    let img = make_image(
        size,
        num_centers,
        num_lookback,
        start_spread,
        cont_spread,
        seed,
    );
    img.save(&filename).expect("saved");
}
