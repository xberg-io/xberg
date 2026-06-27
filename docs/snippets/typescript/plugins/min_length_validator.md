```typescript title="TypeScript"
import { registerValidator, ValidationError, type ExtractedDocument } from "@xberg-io/xberg";

class MinLengthValidator {
  private readonly minLength: number;

  constructor(minLength: number) {
    this.minLength = minLength;
  }

  name(): string {
    return "min-length-validator";
  }

  priority(): number {
    return 100;
  }

  validate(result: ExtractedDocument): void {
    if (result.content.length < this.minLength) {
      throw new ValidationError(
        `Content too short: ${result.content.length} < ${this.minLength} characters`,
      );
    }
  }
}

registerValidator(new MinLengthValidator(50));
```
