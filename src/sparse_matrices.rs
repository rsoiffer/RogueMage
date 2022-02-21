pub(crate) type Entries<'a, K> = Box<dyn Iterator<Item = (K, K, f32)> + 'a>;

pub(crate) type Row<'a, K> = Box<dyn Iterator<Item = (K, f32)> + 'a>;

pub(crate) trait SparseMatrix {
    type Key: Copy;

    fn entries(&self) -> Entries<Self::Key>;

    fn row(&self, source: Self::Key) -> Row<Self::Key>;

    fn get(&self, source: Self::Key, target: Self::Key) -> f32;
}

impl<T: SparseMatrix> SparseMatrix for &T {
    type Key = T::Key;

    fn entries(&self) -> Entries<Self::Key> {
        (*self).entries()
    }

    fn row(&self, source: Self::Key) -> Row<Self::Key> {
        (*self).row(source)
    }

    fn get(&self, source: Self::Key, target: Self::Key) -> f32 {
        (*self).get(source, target)
    }
}

pub(crate) trait TrackingSparseMatrix<S1, S2>
where
    S1: SparseMatrix<Key = Self::Key>,
    S2: SparseMatrix<Key = Self::Key>,
{
    type Key;

    fn current(&self) -> &S1;

    fn previous(&self) -> &S2;
}

pub(crate) fn add<K, S1, S2>(left: S1, right: S2) -> impl SparseMatrix<Key = K>
where
    K: Copy,
    S1: SparseMatrix<Key = K>,
    S2: SparseMatrix<Key = K>,
{
    AddSparseMatrices { left, right }
}

pub(crate) fn sub<K, S1, S2>(left: S1, right: S2) -> impl SparseMatrix<Key = K>
where
    K: Copy,
    S1: SparseMatrix<Key = K>,
    S2: SparseMatrix<Key = K>,
{
    add(left, mul(-1.0, right))
}

pub(crate) fn mul<K, S>(scalar: f32, matrix: S) -> impl SparseMatrix<Key = K>
where
    K: Copy,
    S: SparseMatrix<Key = K>,
{
    MulSparseMatrix { scalar, matrix }
}

pub(crate) fn mat_mul<K, S1, S2>(left: S1, right: S2) -> impl SparseMatrix<Key = K>
where
    K: Copy,
    S1: SparseMatrix<Key = K>,
    S2: SparseMatrix<Key = K>,
{
    MulSparseMatrices { left, right }
}

pub(crate) fn diff<'a, K, S1, S2, T>(matrix: &'a T) -> impl SparseMatrix<Key = K> + 'a
where
    K: Copy + 'a,
    S1: SparseMatrix<Key = K> + 'a,
    S2: SparseMatrix<Key = K> + 'a,
    T: TrackingSparseMatrix<S1, S2, Key = K>,
{
    mul(-1.0, sub(matrix.previous(), matrix.current()))
}

struct AddSparseMatrices<K, S1, S2>
where
    S1: SparseMatrix<Key = K>,
    S2: SparseMatrix<Key = K>,
{
    left: S1,
    right: S2,
}

impl<K, S1, S2> SparseMatrix for AddSparseMatrices<K, S1, S2>
where
    K: Copy,
    S1: SparseMatrix<Key = K>,
    S2: SparseMatrix<Key = K>,
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

struct MulSparseMatrix<K, S>
where
    S: SparseMatrix<Key = K>,
{
    scalar: f32,
    matrix: S,
}

impl<K, S> SparseMatrix for MulSparseMatrix<K, S>
where
    K: Copy,
    S: SparseMatrix<Key = K>,
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
