@0xf79743483a372a8e;

struct AnyValue {
  # The value is one of the listed fields. It is valid for all values to be unspecified
  # in which case this AnyValue is considered to be "empty".
  value :union {
    stringValue @0 :Text;
    boolValue @1 :Bool; 
    intValue @2 :Int64; 
    doubleValue @3 :Float64; 
    arrayValue @4 :ArrayValue; 
    kvlistValue @5 :KeyValueList; 
    bytesValue @6 :Data; 
  }
}

struct ArrayValue {
  values @0 :List(AnyValue);
}

struct KeyValueList {
  values @0 :List(KeyValue);
}

struct KeyValue {
  # The key name of the pair.
  key @0 :Text;

  # The value of the pair.
  value @1 :AnyValue;
}

struct InstrumentationScope {
  name @0 :Text;
  version @1 :Text;
  attributes @2 :List(KeyValue);
  droppedAttributesCount @3 :UInt32;
}

struct EntityRef {
  schemaUrl @0 :Text;
  type @1 :Text;
  idKeys @2 :List(Text);
  descriptionKeys @3 :List(Text);
}
