# Raw ADC Capture Story

A DRV425EVM produces an analog output in response to a sensed magnetic field. An ADC capture system samples that output and produces raw ADC records.

For each captured sample, the system determines whether to accept or reject the resulting raw ADC record.

Accepted raw ADC records are retained together with the acquisition context needed to interpret them. Rejected records remain distinguishable from accepted records.
