use std::cmp::Ordering;

pub struct Expect<T>(pub T);

impl<T> Expect<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> PartialEq<T> for Expect<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &T) -> bool {
        &self.0 == other
    }
}

impl<T> PartialEq for Expect<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        &self.0 == &other.0
    }
}

impl<T> PartialOrd<T> for Expect<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl<T> PartialOrd for Expect<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
