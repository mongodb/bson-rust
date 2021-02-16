/// A BSON array referencing raw bytes stored elsewhere.
pub struct RawArray {
    doc: RawDocumentRef,
}

impl RawArray {
    fn new(data: &[u8]) -> RawResult<&RawArray> {
        Ok(RawArray::from_doc(RawDocumentRef::new(data)?))
    }

    fn from_doc(doc: &RawDocumentRef) -> &RawArray {
        // SAFETY:
        //
        // Dereferencing a raw pointer requires unsafe due to the potential that the pointer is
        // null, dangling, or misaligned. We know the pointer is not null or dangling due to the
        // fact that it's created by a safe reference. Converting &RawDocumentRef to *const
        // RawDocumentRef will be properly aligned due to them being references to the same type,
        // and converting *const RawDocumentRef to *const RawArray is aligned due to the fact that
        // the only field in a RawArray is a RawDocumentRef, meaning the structs are represented
        // identically at the byte level.
        unsafe { &*(doc as *const RawDocumentRef as *const RawArray) }
    }

    /// Gets a reference to the value at the given index.
    pub fn get(&self, index: usize) -> RawResult<Option<RawBson<'_>>> {
        self.into_iter().nth(index).transpose()
    }

    fn get_with<'a, T>(
        &'a self,
        index: usize,
        f: impl FnOnce(elem::RawBson<'a>) -> RawResult<T>,
    ) -> RawResult<Option<T>> {
        self.get(index)?.map(f).transpose()
    }

    /// Gets the BSON double at the given index or returns an error if the value at that index isn't
    /// a double.
    pub fn get_f64(&self, index: usize) -> RawResult<Option<f64>> {
        self.get_with(index, elem::RawBson::as_f64)
    }

    /// Gets a reference to the string at the given index or returns an error if the
    /// value at that index isn't a string.
    pub fn get_str(&self, index: usize) -> RawResult<Option<&str>> {
        self.get_with(index, elem::RawBson::as_str)
    }

    /// Gets a reference to the document at the given index or returns an error if the
    /// value at that index isn't a document.
    pub fn get_document(&self, index: usize) -> RawResult<Option<&RawDocumentRef>> {
        self.get_with(index, elem::RawBson::as_document)
    }

    /// Gets a reference to the array at the given index or returns an error if the
    /// value at that index isn't a array.
    pub fn get_array(&self, index: usize) -> RawResult<Option<&RawArray>> {
        self.get_with(index, elem::RawBson::as_array)
    }

    /// Gets a reference to the BSON binary value at the given index or returns an error if the
    /// value at that index isn't a binary.
    pub fn get_binary(&self, index: usize) -> RawResult<Option<RawBinary<'_>>> {
        self.get_with(index, elem::RawBson::as_binary)
    }

    /// Gets the ObjectId at the given index or returns an error if the value at that index isn't an
    /// ObjectId.
    pub fn get_object_id(&self, index: usize) -> RawResult<Option<ObjectId>> {
        self.get_with(index, elem::RawBson::as_object_id)
    }

    /// Gets the boolean at the given index or returns an error if the value at that index isn't a
    /// boolean.
    pub fn get_bool(&self, index: usize) -> RawResult<Option<bool>> {
        self.get_with(index, elem::RawBson::as_bool)
    }

    /// Gets the DateTime at the given index or returns an error if the value at that index isn't a
    /// DateTime.
    pub fn get_datetime(&self, index: usize) -> RawResult<Option<DateTime<Utc>>> {
        self.get_with(index, elem::RawBson::as_datetime)
    }

    /// Gets a reference to the BSON regex at the given index or returns an error if the
    /// value at that index isn't a regex.
    pub fn get_regex(&self, index: usize) -> RawResult<Option<RawRegex<'_>>> {
        self.get_with(index, elem::RawBson::as_regex)
    }

    /// Gets a reference to the BSON timestamp at the given index or returns an error if the
    /// value at that index isn't a timestamp.
    pub fn get_timestamp(&self, index: usize) -> RawResult<Option<RawTimestamp<'_>>> {
        self.get_with(index, elem::RawBson::as_timestamp)
    }

    /// Gets the BSON int32 at the given index or returns an error if the value at that index isn't
    /// a 32-bit integer.
    pub fn get_i32(&self, index: usize) -> RawResult<Option<i32>> {
        self.get_with(index, elem::RawBson::as_i32)
    }

    /// Gets BSON int64 at the given index or returns an error if the value at that index isn't a
    /// 64-bit integer.
    pub fn get_i64(&self, index: usize) -> RawResult<Option<i64>> {
        self.get_with(index, elem::RawBson::as_i64)
    }

    /// Gets a reference to the raw bytes of the RawArray.
    pub fn as_bytes(&self) -> &[u8] {
        self.doc.as_bytes()
    }
}

impl TryFrom<&RawArray> for Vec<Bson> {
    type Error = Error;

    fn try_from(arr: &RawArray) -> RawResult<Vec<Bson>> {
        arr.into_iter()
            .map(|result| {
                let rawbson = result?;
                Bson::try_from(rawbson)
            })
            .collect()
    }
}

impl<'a> IntoIterator for &'a RawArray {
    type IntoIter = RawArrayIter<'a>;
    type Item = RawResult<elem::RawBson<'a>>;

    fn into_iter(self) -> RawArrayIter<'a> {
        RawArrayIter {
            inner: self.doc.into_iter(),
        }
    }
}

/// An iterator over borrwed raw BSON array values.
pub struct RawArrayIter<'a> {
    inner: RawDocumentIter<'a>,
}

impl<'a> Iterator for RawArrayIter<'a> {
    type Item = RawResult<elem::RawBson<'a>>;

    fn next(&mut self) -> Option<RawResult<RawBson<'a>>> {
        match self.inner.next() {
            Some(Ok((_, v))) => Some(Ok(v)),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}
