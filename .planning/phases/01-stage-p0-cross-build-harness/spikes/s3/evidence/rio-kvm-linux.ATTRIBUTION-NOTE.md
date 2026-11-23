# S3 rio witness log: attribution note (owner clarification, 2026-11-23)

The committed excerpt `rio-kvm-linux.log` contains this line:

```
 Feb  7 01:28:16 tr-ok glenda@glenda(unknown) -> glenda@bootes
```

That line is **not** evidence from the gate's headless drawterm. The owner
clarified on 2026-11-23, verbatim:

> Recommendation 5 (D-11 rio marker): keep the two `/srv/rio.*` and `/srv/riowctl.*` entries as the marker, but the owner clarifies the s3-rio failure evidence: the `tr-ok glenda@glenda(unknown) -> glenda@bootes` line came from the owner's own graphical drawterm login attempt while the run appeared hung, not from the gate's headless drawterm. The owner did not know the generated DEMO password for glenda. The owner believes the headless drawterm login attempt did not work at all. So the positive leg is unproven because headless authentication/password feed never succeeded, not merely because of an expect prompt mismatch. Next action: determine the generated DEMO password/credential path, fix the headless drawterm auth and password feed, then re-run the rio leg; do not treat the `tr-ok` line as gate evidence.

## What this changes

- The gate's headless drawterm started under Xvfb but never authenticated:
  the generated DEMO password for glenda was not known, so the password feed
  never succeeded.
- The `tr-ok` line records the owner's manual **graphical** drawterm login
  attempt made while the run appeared hung. It must not be cited as a gate
  authentication, an A6 confirmation, or evidence that the headless path works.
- The `s3-rio-kvm-linux` verdict remains **FAIL**. The positive leg is unproven
  because headless authentication and the password feed never succeeded.
- The next action is to determine the generated DEMO password/credential path,
  fix the headless drawterm auth and password feed, then re-run the rio leg;
  keep `control-noG` unchanged as the negative control.

The raw captured lines in `rio-kvm-linux.log` are unchanged. The log's header
comment and one curator comment were corrected to point at this note; no
captured output was altered.
