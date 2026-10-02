# Preserved v4 admission failure

The frozen v4 schedule stopped before full mutation admission. Its fresh C2 cost
verification completed with the expected rejection in 1663.367451541999 monotonic
seconds; the original command, source binary, profile and streams are identified
in the retained command receipt. The exact completed diagnostic was then retired
successfully. The immediately following two-generation reservation failed because
observed free space was below its unchanged 33350717651-byte requirement.

The failed reservation did not save its exact free-byte reading. Its prior sample
and a separate later observation are retained explicitly as different observations.
Root planning had assumed immediate physical-space availability from logical file
retirement. That assumption was unsupported. No filesystem cause is asserted.

The coordinator remains failed. Cleanup passed and both child suites exited1.
Both suite status values are `input-changed`: termination interrupted their final
input-hash pass after the admission failure. Those incomplete final checks receive
no acceptance credit. The successful native C2 command has its own before/after
input checks and raw terminal rejection; a later selected-case admission must
independently authenticate it before reuse.

The source-reviewed v4 final checker was never launched. Its prerequisite failure
is preserved. A separate sequential v5 preparation will reserve actual space for
one mutation at a time, with bounded observation waits and unchanged native limits.
No remaining mutation result or SH8 acceptance is claimed here.
