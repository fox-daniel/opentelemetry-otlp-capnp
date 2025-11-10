@0xce505d6ac134e147;


using Common = import "../../common/v1/common.capnp";

struct Resource {
  attributes @0 :List(Common.KeyValue);
  droppedAttributesCount @1 :UInt32;
  entityRefs @2 :List(Common.EntityRef); 
}
