---
hide:
  - navigation
---

# FAQ

### Is Trident supported only with Anchor ?

- Currently yes, Trident under the hood obtains data from the IDL generated by Anchor.


### I created the Fuzz Test what should I do next ?

- Start here [Writing Fuzz Tests](../writing-fuzz-test/writing-fuzz-test.md). For additional features check [Features](../features/features.md). If you are not sure about anything check [Get Help](../get-help/get-help.md)


### My program Instruction contains custom type such as Struct or Enum on its input, but it does not derive Arbitrary.

- In this case you need to specify same type in the Fuzz Test (with the same fields). And implement From Trait to convert to your type. Check [Custom Data Types](../features/customize-ix-data.md) or [Examples of Arbitrary](../examples/examples.md).


### Is Trident open-source ?

- Yes, here [Trident](https://github.com/Ackee-Blockchain/trident)

### I would like to report Issue with Trident, what should I do ?

- Write Issue [Issues](https://github.com/Ackee-Blockchain/trident/issues)

### Is Trident deployed on Mainnet / Devnet / Testenet ?

- No, Trident is Fuzz Testing Framework, not Solana Program.

### What type of Fuzzer Trident is ?

- Currently, we refer to it as *"coverage guided gray box fuzzer"*.
