## Internal lossless serialization shape for Path.
PathEncoding := [Utf8(Str), UnixBytes(List(U8)), WindowsU16s(List(U16))].{
	parser_for : _
	encoder_for : _
}

## The generic tagged codec preserves invalid native units.
expect {
	original = PathEncoding.WindowsU16s([0xD800, 97])
	decoded : Try(PathEncoding, Json.ParseErr)
	decoded = Json.parse(Json.to_str(original))
	decoded == Ok(original)
}
