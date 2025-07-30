# Testing Binary subtype 9: Vector

The JSON files in this directory tree are platform-independent tests that drivers can use to prove their conformance to
the specification.

These tests focus on the roundtrip of the list of numbers as input/output, along with their data type and byte padding.

Additional tests exist in `bson_corpus/tests/binary.json` but do not sufficiently test the end-to-end process of Vector
to BSON. For this reason, drivers must create a bespoke test runner for the vector subtype.

## Format

The test data corpus consists of a JSON file for each data type (dtype). Each file contains a number of test cases,
under the top-level key "tests". Each test case pertains to a single vector. The keys provide the specification of the
vector. Valid cases also include the Canonical BSON format of a document {test_key: binary}. The "test_key" is common,
and specified at the top level.

#### Top level keys

Each JSON file contains three top-level keys.

- `description`: human-readable description of what is in the file
- `test_key`: name used for key when encoding/decoding a BSON document containing the single BSON Binary for the test
    case. Applies to *every* case.
- `tests`: array of test case objects, each of which have the following keys. Valid cases will also contain additional
    binary and json encoding values.

#### Keys of individual tests cases

- `description`: string describing the test.
- `valid`: boolean indicating if the vector, dtype, and padding should be considered a valid input.
- `vector`: (required if valid is true) list of numbers
- `dtype_hex`: string defining the data type in hex (e.g. "0x10", "0x27")
- `dtype_alias`: (optional) string defining the data dtype, perhaps as Enum.
- `padding`: (optional) integer for byte padding. Defaults to 0.
- `canonical_bson`: (required if valid is true) an (uppercase) big-endian hex representation of a BSON byte string.

## Required tests

#### To prove correct in a valid case (`valid: true`), one MUST

- encode a document from the numeric values, dtype, and padding, along with the "test_key", and assert this matches the
    canonical_bson string.
- decode the canonical_bson into its binary form, and then assert that the numeric values, dtype, and padding all match
    those provided in the JSON.

Note: For floating point number types, exact numerical matches may not be possible. Drivers that natively support the
floating-point type being tested (e.g., when testing float32 vector values in a driver that natively supports float32),
MUST assert that the input float array is the same after encoding and decoding.

#### To prove correct in an invalid case (`valid:false`), one MUST

- if the vector field is present, raise an exception when attempting to encode a document from the numeric values,
    dtype, and padding.
- if the canonical_bson field is present, raise an exception when attempting to deserialize it into the corresponding
    numeric values, as the field contains corrupted data.

## Prose Tests

### Treatment of non-zero ignored bits

All drivers MUST test encoding and decoding behavior according to their design and version. For drivers that haven't
been completed, raise exceptions in both cases. For those that have, update to this behavior according to semantic
versioning rules, and update tests accordingly.

In both cases, [255], a single byte PACKED_BIT vector of length 1 (hence padding of 7) provides a good example to use,
as all of its bits are ones.

#### 1. Encoding

- Test encoding with non-zero ignored bits. Use the driver API that validates vector metadata.
- If the driver validates ignored bits are zero (preferred), expect an error. Otherwise expect the ignored bits are
    preserved.

```python
with pytest.raises(ValueError):
    Binary.from_vector([0b11111111], BinaryVectorDtype.PACKED_BIT, padding=7)
```

### 2. Decoding

- Test the behaviour of your driver when one attempts to decode from binary to vector.
    - e.g. As of pymongo 4.14, a warning is raised. From 5.0, it will be an exception.

```python
b = Binary(b'\x10\x07\xff', subtype=9)
with pytest.warns():
    Binary.as_vector(b)
```

Drivers MAY skip this test if they choose not to implement a `Vector` type.

### 3. Comparison

Once we can guarantee that all ignored bits are non-zero, then equality can be tested on the binary subtype. Until then,
equality is ambiguous, and depends on whether one compares by bits (uint1), or uint8. Drivers SHOULD test equality
behavior according to their design and version.

For example, in `pymongo < 5.0`, we define equality of a BinaryVector by matching padding, dtype, and integer. This
means that two single bit vectors in which 7 bits are ignored do not match unless all bits match. This mirrors what the
server does.

```python
b1 = Binary(b'\x10\x07\x80', subtype=9) # 1-bit vector with all 0 ignored bits.
b2 = Binary(b'\x10\x07\xff', subtype=9) # 1-bit vector with all 1 ignored bits.
b3 = Binary.from_vector([0b10000000], BinaryVectorDtype.PACKED_BIT, padding=7) # Same data as b1.

v1 = Binary.as_vector(b1)
v2 = Binary.as_vector(b2)
v3 = Binary.as_vector(b3)

assert b1 != b2  # Unequal at naive Binary level 
assert v2 != v1  # Also chosen to be unequal at BinaryVector level as [255] != [128]
assert b1 == b3  # Equal at naive Binary level
assert v1 == v3  # Equal at the BinaryVector level
```

Drivers MAY skip this test if they choose not to implement a `Vector` type, or the type does not support comparison, or
the type cannot be constructed with non-zero ignored bits.

## FAQ

- What MongoDB Server version does this apply to?
    - Files in the "specifications" repository have no version scheme. They are not tied to a MongoDB server version.
