```typescript title="TypeScript"
import { registerValidator, ValidationError, type ExtractedDocument } from "@xberg-io/xberg";

class QualityScoreValidator {
  private readonly minScore: number;

  constructor(minScore: number = 0.5) {
    this.minScore = minScore;
  }

  name(): string {
    return "quality-score-validator";
  }

  priority(): number {
    return 50;
  }

  validate(result: ExtractedDocument): void {
    const score = Number(result.metadata?.qualityScore ?? 0);

    if (score < this.minScore) {
      throw new ValidationError(
        `Quality score too low: ${score.toFixed(2)} < ${this.minScore.toFixed(2)}`,
      );
    }
  }
}

registerValidator(new QualityScoreValidator(0.5));
```
