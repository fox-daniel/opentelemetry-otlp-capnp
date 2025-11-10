@0xb89b2039eddecd64;

using Trace = import "../../../trace/v1/trace.capnp";

interface TraceService {
     exportTraces @0 (request: ExportTraceServiceRequest) -> (response: ExportTraceServiceResponse);
   }

struct ExportTraceServiceRequest {
     resourceSpans @0 :List(Trace.ResourceSpans);
}

struct ExportTraceServiceResponse {
     partialSuccess @0 :ExportTracePartialSuccess;
}

struct ExportTracePartialSuccess {
     rejectedSpans @0 :Int64;
     errorMessage @1 :Text;
}
