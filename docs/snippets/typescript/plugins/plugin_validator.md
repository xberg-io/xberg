```ts title="TypeScript"
import {
  extractSync,
  registerValidator,
  unregisterValidator,
  ValidationError,
  type ExtractionResult,
} from "@xberg-io/xberg";

class MinLengthValidator {
  name = "min_length_validator";
  priority = 10;

  validate(result: ExtractionResult): void {
    if (result.content.length < 50) {
      throw new ValidationError(`Content too short: ${result.content.length}`);
    }
  }
}

registerValidator(new MinLengthValidator());

const result = extractSync("document.pdf");
console.log(`Validated content length: ${result.content.length}`);

unregisterValidator("min_length_validator");
```
