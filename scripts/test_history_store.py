import json
import unittest
from unittest.mock import patch
import history_store as h

class HistoryTests(unittest.TestCase):
    def context(self):
        return {"event_id":"now","event_started_at_ms":1000,"history_scope":"experiment-a","primary":{"captured_at_ms":900,"data_kind":"synthetic"},"fusion":{"fused_ppm":798.0}}

    def record(self,**changes):
        result={"candidateJson":json.dumps({"source_event_id":"past"}),"eventRecordJson":"{}","eventId":"past","resolvedAt":800,"historyScope":"experiment-a","dataKind":"synthetic","_additional":{"id":"opaque-id","distance":0.2}}
        result.update(changes)
        return result

    def test_real_query_uses_fused_value_top_five_and_past_resolution_filter(self):
        with patch.object(h,"request",return_value={"data":{"Get":{h.CLASS:[self.record()]}}}) as request:
            records,provenance=h.retrieve(self.context(),1000)
        query=request.call_args.args[2]["query"]
        self.assertIn("nearText",query)
        self.assertIn("798.000000",query)
        self.assertIn("limit:5",query)
        self.assertIn("LessThan,valueNumber:1000",query)
        self.assertIn("historyScope",query)
        self.assertEqual(records,[{"source_event_id":"past"}])
        self.assertEqual(provenance["records"][0]["eventId"],"past")

    def test_current_future_and_wrong_modality_or_scope_are_rejected(self):
        for changes in [{"eventId":"now"},{"resolvedAt":1000},{"resolvedAt":1100},{"dataKind":"physical"},{"historyScope":"other-experiment"}]:
            with self.subTest(changes=changes),patch.object(h,"request",return_value={"data":{"Get":{h.CLASS:[self.record(**changes)]}}}):
                with self.assertRaises(ValueError):
                    h.retrieve(self.context(),1000)

    def test_service_errors_and_excess_results_are_not_successful_empty_history(self):
        for response in [{"errors":[{"message":"offline"}]},{"data":{"Get":{h.CLASS:[self.record()]*6}}}]:
            with patch.object(h,"request",return_value=response):
                with self.assertRaises(ValueError):
                    h.retrieve(self.context(),1000)

if __name__=="__main__":
    unittest.main()
