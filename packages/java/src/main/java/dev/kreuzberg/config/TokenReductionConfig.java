package dev.kreuzberg.config;

import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.ValidationHelper;
import java.util.HashMap;
import java.util.Map;

/**
 * Token reduction configuration for minimizing output size.
 *
 * @since 4.0.0
 */
public final class TokenReductionConfig {
  private final String mode;
  private final boolean preserveImportantWords;

  private TokenReductionConfig(Builder builder) {
    this.mode = builder.mode;
    this.preserveImportantWords = builder.preserveImportantWords;
  }

  public static Builder builder() {
    return new Builder();
  }

  public String getMode() {
    return mode;
  }

  public boolean isPreserveImportantWords() {
    return preserveImportantWords;
  }

  public Map<String, Object> toMap() {
    Map<String, Object> map = new HashMap<>();
    map.put("mode", mode);
    map.put("preserve_important_words", preserveImportantWords);
    return map;
  }

  public static final class Builder {
    private String mode = "off";
    private boolean preserveImportantWords = true;

    private Builder() {
    }

    public Builder mode(String mode) {
      try {
        ValidationHelper.validateTokenReductionLevel(mode);
      } catch (KreuzbergException e) {
        throw new IllegalArgumentException(e.getMessage(), e);
      }
      this.mode = mode;
      return this;
    }

    public Builder preserveImportantWords(boolean preserveImportantWords) {
      this.preserveImportantWords = preserveImportantWords;
      return this;
    }

    public TokenReductionConfig build() {
      return new TokenReductionConfig(this);
    }
  }

  static TokenReductionConfig fromMap(Map<String, Object> map) {
    if (map == null) {
      return null;
    }
    Builder builder = builder();
    if (map.get("mode") instanceof String) {
      builder.mode((String) map.get("mode"));
    }
    if (map.get("preserve_important_words") instanceof Boolean) {
      builder.preserveImportantWords((Boolean) map.get("preserve_important_words"));
    }
    return builder.build();
  }
}
