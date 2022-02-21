pub(crate) type Entries<'a, K> = Box<dyn Iterator<Item = (K, K, f32)> + 'a>;

pub(crate) type Row<'a, K> = Box<dyn Iterator<Item = (K, f32)> + 'a>;

pub(crate) trait SparseMatrix {
    type Key: Copy;
    fn entries(&self) -> Entries<Self::Key>;
    fn row(&self, source: Self::Key) -> Row<Self::Key>;
    fn get(&self, source: Self::Key, target: Self::Key) -> f32;
}

pub(crate) trait TrackingSparseMatrix<'a, S1, S2>
where
    S1: SparseMatrix<Key = Self::Key>,
    S2: SparseMatrix<Key = Self::Key>,
{
    type Key;
    fn current(&'a self) -> &'a S1;
    fn previous(&'a self) -> &'a S2;
}

pub(crate) fn add<'a, K, S1, S2>(left: &'a S1, right: &'a S2) -> impl SparseMatrix<Key = K> + 'a
where
    S1: SparseMatrix<Key = K>,
    S2: SparseMatrix<Key = K>,
    K: Copy + 'a,
{
    AddSparseMatrices { left, right }
}

pub(crate) fn sub<'a, K, S1, S2>(left: &'a S1, right: &'a S2) -> impl SparseMatrix<Key = K> + 'a
where
    S1: SparseMatrix<Key = K>,
    S2: SparseMatrix<Key = K>,
    K: Copy + 'a,
{
    add::<'a>(left, &mul::<'a>(-1.0, right))
}

pub(crate) fn mul<'a, K, S>(scalar: f32, matrix: &'a S) -> impl SparseMatrix<Key = K> + 'a
where
    S: SparseMatrix<Key = K>,
    K: Copy + 'a,
{
    MulSparseMatrix { scalar, matrix }
}

pub(crate) fn mat_mul<'a, K, S1, S2>(left: &'a S1, right: &'a S2) -> impl SparseMatrix<Key = K> + 'a
where
    S1: SparseMatrix<Key = K>,
    S2: SparseMatrix<Key = K>,
    K: Copy,
{
    MulSparseMatrices { left, right }
}

pub(crate) fn diff<'a, K, S, S1, S2>(matrix: &'a S) -> impl SparseMatrix<Key = K>
where
    S: TrackingSparseMatrix<'a, S1, S2, Key = K> + 'a,
    S1: SparseMatrix<Key = K> + 'a,
    S2: SparseMatrix<Key = K> + 'a,
    K: Copy,
{
    mul(-1.0, sub(matrix.previous(), matrix.current()))
}

struct AddSparseMatrices<'a, K, S1, S2>
where
    S1: SparseMatrix<Key = K>,
    S2: SparseMatrix<Key = K>,
{
    left: &'a S1,
    right: &'a S2,
}

impl<'a, K, S1, S2> SparseMatrix for AddSparseMatrices<'a, K, S1, S2>
where
    S1: SparseMatrix<Key = K>,
    S2: SparseMatrix<Key = K>,
    K: Copy,
{
    type Key = K;

    fn entries(&self) -> Entries<Self::Key> {
        Box::new(
            self.left.entries().map(|(source, target, val)| {
                (source, target, val + self.right.get(source, target))
            }),
        )
    }

    fn row(&self, source: Self::Key) -> Row<Self::Key> {
        Box::new(
            self.left
                .row(source)
                .map(move |(target, val)| (target, val + self.right.get(source, target))),
        )
    }

    fn get(&self, source: Self::Key, target: Self::Key) -> f32 {
        self.left.get(source, target) + self.right.get(source, target)
    }
}

struct MulSparseMatrix<'a, K, S>
where
    S: SparseMatrix<Key = K>,
{
    scalar: f32,
    matrix: &'a S,
}

impl<'a, K, S> SparseMatrix for MulSparseMatrix<'a, K, S>
where
    S: SparseMatrix<Key = K>,
    K: Copy,
{
    type Key = K;

    fn entries(&self) -> Entries<Self::Key> {
        Box::new(
            self.matrix
                .entries()
                .map(|(source, target, val)| (source, target, self.scalar * val)),
        )
    }

    fn row(&self, source: Self::Key) -> Row<Self::Key> {
        Box::new(
            self.matrix
                .row(source)
                .map(|(target, val)| (target, self.scalar * val)),
        )
    }

    fn get(&self, source: Self::Key, target: Self::Key) -> f32 {
        self.scalar * self.matrix.get(source, target)
    }
}

struct MulSparseMatrices<K, S1, S2>
where
    S1: SparseMatrix<Key = K>,
    S2: SparseMatrix<Key = K>,
{
    left: S1,
    right: S2,
}

impl<K, S1, S2> SparseMatrix for MulSparseMatrices<K, S1, S2>
where
    S1: SparseMatrix<Key = K>,
    S2: SparseMatrix<Key = K>,
    K: Copy,
{
    type Key = K;

    fn entries(&self) -> Entries<Self::Key> {
        Box::new(
            self.left
                .entries()
                .flat_map(move |(source, middle, value1)| {
                    let iter = self.right.row(middle);
                    iter.map(move |(target, value2)| (source, target, value1 * value2))
                }),
        )
    }

    fn row(&self, source: Self::Key) -> Row<Self::Key> {
        todo!()
    }

    fn get(&self, source: Self::Key, target: Self::Key) -> f32 {
        todo!()
    }
}
