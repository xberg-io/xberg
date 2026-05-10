// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'lib.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;

/// @nodoc
mixin _$AnnotationKind {
  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is AnnotationKind);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'AnnotationKind()';
  }
}

/// @nodoc
class $AnnotationKindCopyWith<$Res> {
  $AnnotationKindCopyWith(AnnotationKind _, $Res Function(AnnotationKind) __);
}

/// Adds pattern-matching-related methods to [AnnotationKind].
extension AnnotationKindPatterns on AnnotationKind {
  /// A variant of `map` that fallback to returning `orElse`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(AnnotationKind_Bold value)? bold,
    TResult Function(AnnotationKind_Italic value)? italic,
    TResult Function(AnnotationKind_Underline value)? underline,
    TResult Function(AnnotationKind_Strikethrough value)? strikethrough,
    TResult Function(AnnotationKind_Code value)? code,
    TResult Function(AnnotationKind_Subscript value)? subscript,
    TResult Function(AnnotationKind_Superscript value)? superscript,
    TResult Function(AnnotationKind_Link value)? link,
    TResult Function(AnnotationKind_Highlight value)? highlight,
    TResult Function(AnnotationKind_Color value)? color,
    TResult Function(AnnotationKind_FontSize value)? fontSize,
    TResult Function(AnnotationKind_Custom value)? custom,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case AnnotationKind_Bold() when bold != null:
        return bold(_that);
      case AnnotationKind_Italic() when italic != null:
        return italic(_that);
      case AnnotationKind_Underline() when underline != null:
        return underline(_that);
      case AnnotationKind_Strikethrough() when strikethrough != null:
        return strikethrough(_that);
      case AnnotationKind_Code() when code != null:
        return code(_that);
      case AnnotationKind_Subscript() when subscript != null:
        return subscript(_that);
      case AnnotationKind_Superscript() when superscript != null:
        return superscript(_that);
      case AnnotationKind_Link() when link != null:
        return link(_that);
      case AnnotationKind_Highlight() when highlight != null:
        return highlight(_that);
      case AnnotationKind_Color() when color != null:
        return color(_that);
      case AnnotationKind_FontSize() when fontSize != null:
        return fontSize(_that);
      case AnnotationKind_Custom() when custom != null:
        return custom(_that);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// Callbacks receives the raw object, upcasted.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case final Subclass2 value:
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(AnnotationKind_Bold value) bold,
    required TResult Function(AnnotationKind_Italic value) italic,
    required TResult Function(AnnotationKind_Underline value) underline,
    required TResult Function(AnnotationKind_Strikethrough value) strikethrough,
    required TResult Function(AnnotationKind_Code value) code,
    required TResult Function(AnnotationKind_Subscript value) subscript,
    required TResult Function(AnnotationKind_Superscript value) superscript,
    required TResult Function(AnnotationKind_Link value) link,
    required TResult Function(AnnotationKind_Highlight value) highlight,
    required TResult Function(AnnotationKind_Color value) color,
    required TResult Function(AnnotationKind_FontSize value) fontSize,
    required TResult Function(AnnotationKind_Custom value) custom,
  }) {
    final _that = this;
    switch (_that) {
      case AnnotationKind_Bold():
        return bold(_that);
      case AnnotationKind_Italic():
        return italic(_that);
      case AnnotationKind_Underline():
        return underline(_that);
      case AnnotationKind_Strikethrough():
        return strikethrough(_that);
      case AnnotationKind_Code():
        return code(_that);
      case AnnotationKind_Subscript():
        return subscript(_that);
      case AnnotationKind_Superscript():
        return superscript(_that);
      case AnnotationKind_Link():
        return link(_that);
      case AnnotationKind_Highlight():
        return highlight(_that);
      case AnnotationKind_Color():
        return color(_that);
      case AnnotationKind_FontSize():
        return fontSize(_that);
      case AnnotationKind_Custom():
        return custom(_that);
    }
  }

  /// A variant of `map` that fallback to returning `null`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(AnnotationKind_Bold value)? bold,
    TResult? Function(AnnotationKind_Italic value)? italic,
    TResult? Function(AnnotationKind_Underline value)? underline,
    TResult? Function(AnnotationKind_Strikethrough value)? strikethrough,
    TResult? Function(AnnotationKind_Code value)? code,
    TResult? Function(AnnotationKind_Subscript value)? subscript,
    TResult? Function(AnnotationKind_Superscript value)? superscript,
    TResult? Function(AnnotationKind_Link value)? link,
    TResult? Function(AnnotationKind_Highlight value)? highlight,
    TResult? Function(AnnotationKind_Color value)? color,
    TResult? Function(AnnotationKind_FontSize value)? fontSize,
    TResult? Function(AnnotationKind_Custom value)? custom,
  }) {
    final _that = this;
    switch (_that) {
      case AnnotationKind_Bold() when bold != null:
        return bold(_that);
      case AnnotationKind_Italic() when italic != null:
        return italic(_that);
      case AnnotationKind_Underline() when underline != null:
        return underline(_that);
      case AnnotationKind_Strikethrough() when strikethrough != null:
        return strikethrough(_that);
      case AnnotationKind_Code() when code != null:
        return code(_that);
      case AnnotationKind_Subscript() when subscript != null:
        return subscript(_that);
      case AnnotationKind_Superscript() when superscript != null:
        return superscript(_that);
      case AnnotationKind_Link() when link != null:
        return link(_that);
      case AnnotationKind_Highlight() when highlight != null:
        return highlight(_that);
      case AnnotationKind_Color() when color != null:
        return color(_that);
      case AnnotationKind_FontSize() when fontSize != null:
        return fontSize(_that);
      case AnnotationKind_Custom() when custom != null:
        return custom(_that);
      case _:
        return null;
    }
  }

  /// A variant of `when` that fallback to an `orElse` callback.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? bold,
    TResult Function()? italic,
    TResult Function()? underline,
    TResult Function()? strikethrough,
    TResult Function()? code,
    TResult Function()? subscript,
    TResult Function()? superscript,
    TResult Function(String url, String title)? link,
    TResult Function()? highlight,
    TResult Function(String value)? color,
    TResult Function(String value)? fontSize,
    TResult Function(String name, String value)? custom,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case AnnotationKind_Bold() when bold != null:
        return bold();
      case AnnotationKind_Italic() when italic != null:
        return italic();
      case AnnotationKind_Underline() when underline != null:
        return underline();
      case AnnotationKind_Strikethrough() when strikethrough != null:
        return strikethrough();
      case AnnotationKind_Code() when code != null:
        return code();
      case AnnotationKind_Subscript() when subscript != null:
        return subscript();
      case AnnotationKind_Superscript() when superscript != null:
        return superscript();
      case AnnotationKind_Link() when link != null:
        return link(_that.url, _that.title);
      case AnnotationKind_Highlight() when highlight != null:
        return highlight();
      case AnnotationKind_Color() when color != null:
        return color(_that.value);
      case AnnotationKind_FontSize() when fontSize != null:
        return fontSize(_that.value);
      case AnnotationKind_Custom() when custom != null:
        return custom(_that.name, _that.value);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// As opposed to `map`, this offers destructuring.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case Subclass2(:final field2):
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() bold,
    required TResult Function() italic,
    required TResult Function() underline,
    required TResult Function() strikethrough,
    required TResult Function() code,
    required TResult Function() subscript,
    required TResult Function() superscript,
    required TResult Function(String url, String title) link,
    required TResult Function() highlight,
    required TResult Function(String value) color,
    required TResult Function(String value) fontSize,
    required TResult Function(String name, String value) custom,
  }) {
    final _that = this;
    switch (_that) {
      case AnnotationKind_Bold():
        return bold();
      case AnnotationKind_Italic():
        return italic();
      case AnnotationKind_Underline():
        return underline();
      case AnnotationKind_Strikethrough():
        return strikethrough();
      case AnnotationKind_Code():
        return code();
      case AnnotationKind_Subscript():
        return subscript();
      case AnnotationKind_Superscript():
        return superscript();
      case AnnotationKind_Link():
        return link(_that.url, _that.title);
      case AnnotationKind_Highlight():
        return highlight();
      case AnnotationKind_Color():
        return color(_that.value);
      case AnnotationKind_FontSize():
        return fontSize(_that.value);
      case AnnotationKind_Custom():
        return custom(_that.name, _that.value);
    }
  }

  /// A variant of `when` that fallback to returning `null`
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? bold,
    TResult? Function()? italic,
    TResult? Function()? underline,
    TResult? Function()? strikethrough,
    TResult? Function()? code,
    TResult? Function()? subscript,
    TResult? Function()? superscript,
    TResult? Function(String url, String title)? link,
    TResult? Function()? highlight,
    TResult? Function(String value)? color,
    TResult? Function(String value)? fontSize,
    TResult? Function(String name, String value)? custom,
  }) {
    final _that = this;
    switch (_that) {
      case AnnotationKind_Bold() when bold != null:
        return bold();
      case AnnotationKind_Italic() when italic != null:
        return italic();
      case AnnotationKind_Underline() when underline != null:
        return underline();
      case AnnotationKind_Strikethrough() when strikethrough != null:
        return strikethrough();
      case AnnotationKind_Code() when code != null:
        return code();
      case AnnotationKind_Subscript() when subscript != null:
        return subscript();
      case AnnotationKind_Superscript() when superscript != null:
        return superscript();
      case AnnotationKind_Link() when link != null:
        return link(_that.url, _that.title);
      case AnnotationKind_Highlight() when highlight != null:
        return highlight();
      case AnnotationKind_Color() when color != null:
        return color(_that.value);
      case AnnotationKind_FontSize() when fontSize != null:
        return fontSize(_that.value);
      case AnnotationKind_Custom() when custom != null:
        return custom(_that.name, _that.value);
      case _:
        return null;
    }
  }
}

/// @nodoc

class AnnotationKind_Bold extends AnnotationKind {
  const AnnotationKind_Bold() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is AnnotationKind_Bold);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'AnnotationKind.bold()';
  }
}

/// @nodoc

class AnnotationKind_Italic extends AnnotationKind {
  const AnnotationKind_Italic() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is AnnotationKind_Italic);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'AnnotationKind.italic()';
  }
}

/// @nodoc

class AnnotationKind_Underline extends AnnotationKind {
  const AnnotationKind_Underline() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is AnnotationKind_Underline);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'AnnotationKind.underline()';
  }
}

/// @nodoc

class AnnotationKind_Strikethrough extends AnnotationKind {
  const AnnotationKind_Strikethrough() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is AnnotationKind_Strikethrough);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'AnnotationKind.strikethrough()';
  }
}

/// @nodoc

class AnnotationKind_Code extends AnnotationKind {
  const AnnotationKind_Code() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is AnnotationKind_Code);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'AnnotationKind.code()';
  }
}

/// @nodoc

class AnnotationKind_Subscript extends AnnotationKind {
  const AnnotationKind_Subscript() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is AnnotationKind_Subscript);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'AnnotationKind.subscript()';
  }
}

/// @nodoc

class AnnotationKind_Superscript extends AnnotationKind {
  const AnnotationKind_Superscript() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is AnnotationKind_Superscript);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'AnnotationKind.superscript()';
  }
}

/// @nodoc

class AnnotationKind_Link extends AnnotationKind {
  const AnnotationKind_Link({required this.url, required this.title})
      : super._();

  final String url;
  final String title;

  /// Create a copy of AnnotationKind
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $AnnotationKind_LinkCopyWith<AnnotationKind_Link> get copyWith =>
      _$AnnotationKind_LinkCopyWithImpl<AnnotationKind_Link>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is AnnotationKind_Link &&
            (identical(other.url, url) || other.url == url) &&
            (identical(other.title, title) || other.title == title));
  }

  @override
  int get hashCode => Object.hash(runtimeType, url, title);

  @override
  String toString() {
    return 'AnnotationKind.link(url: $url, title: $title)';
  }
}

/// @nodoc
abstract mixin class $AnnotationKind_LinkCopyWith<$Res>
    implements $AnnotationKindCopyWith<$Res> {
  factory $AnnotationKind_LinkCopyWith(
          AnnotationKind_Link value, $Res Function(AnnotationKind_Link) _then) =
      _$AnnotationKind_LinkCopyWithImpl;
  @useResult
  $Res call({String url, String title});
}

/// @nodoc
class _$AnnotationKind_LinkCopyWithImpl<$Res>
    implements $AnnotationKind_LinkCopyWith<$Res> {
  _$AnnotationKind_LinkCopyWithImpl(this._self, this._then);

  final AnnotationKind_Link _self;
  final $Res Function(AnnotationKind_Link) _then;

  /// Create a copy of AnnotationKind
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? url = null,
    Object? title = null,
  }) {
    return _then(AnnotationKind_Link(
      url: null == url
          ? _self.url
          : url // ignore: cast_nullable_to_non_nullable
              as String,
      title: null == title
          ? _self.title
          : title // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class AnnotationKind_Highlight extends AnnotationKind {
  const AnnotationKind_Highlight() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is AnnotationKind_Highlight);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'AnnotationKind.highlight()';
  }
}

/// @nodoc

class AnnotationKind_Color extends AnnotationKind {
  const AnnotationKind_Color({required this.value}) : super._();

  final String value;

  /// Create a copy of AnnotationKind
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $AnnotationKind_ColorCopyWith<AnnotationKind_Color> get copyWith =>
      _$AnnotationKind_ColorCopyWithImpl<AnnotationKind_Color>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is AnnotationKind_Color &&
            (identical(other.value, value) || other.value == value));
  }

  @override
  int get hashCode => Object.hash(runtimeType, value);

  @override
  String toString() {
    return 'AnnotationKind.color(value: $value)';
  }
}

/// @nodoc
abstract mixin class $AnnotationKind_ColorCopyWith<$Res>
    implements $AnnotationKindCopyWith<$Res> {
  factory $AnnotationKind_ColorCopyWith(AnnotationKind_Color value,
          $Res Function(AnnotationKind_Color) _then) =
      _$AnnotationKind_ColorCopyWithImpl;
  @useResult
  $Res call({String value});
}

/// @nodoc
class _$AnnotationKind_ColorCopyWithImpl<$Res>
    implements $AnnotationKind_ColorCopyWith<$Res> {
  _$AnnotationKind_ColorCopyWithImpl(this._self, this._then);

  final AnnotationKind_Color _self;
  final $Res Function(AnnotationKind_Color) _then;

  /// Create a copy of AnnotationKind
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? value = null,
  }) {
    return _then(AnnotationKind_Color(
      value: null == value
          ? _self.value
          : value // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class AnnotationKind_FontSize extends AnnotationKind {
  const AnnotationKind_FontSize({required this.value}) : super._();

  final String value;

  /// Create a copy of AnnotationKind
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $AnnotationKind_FontSizeCopyWith<AnnotationKind_FontSize> get copyWith =>
      _$AnnotationKind_FontSizeCopyWithImpl<AnnotationKind_FontSize>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is AnnotationKind_FontSize &&
            (identical(other.value, value) || other.value == value));
  }

  @override
  int get hashCode => Object.hash(runtimeType, value);

  @override
  String toString() {
    return 'AnnotationKind.fontSize(value: $value)';
  }
}

/// @nodoc
abstract mixin class $AnnotationKind_FontSizeCopyWith<$Res>
    implements $AnnotationKindCopyWith<$Res> {
  factory $AnnotationKind_FontSizeCopyWith(AnnotationKind_FontSize value,
          $Res Function(AnnotationKind_FontSize) _then) =
      _$AnnotationKind_FontSizeCopyWithImpl;
  @useResult
  $Res call({String value});
}

/// @nodoc
class _$AnnotationKind_FontSizeCopyWithImpl<$Res>
    implements $AnnotationKind_FontSizeCopyWith<$Res> {
  _$AnnotationKind_FontSizeCopyWithImpl(this._self, this._then);

  final AnnotationKind_FontSize _self;
  final $Res Function(AnnotationKind_FontSize) _then;

  /// Create a copy of AnnotationKind
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? value = null,
  }) {
    return _then(AnnotationKind_FontSize(
      value: null == value
          ? _self.value
          : value // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class AnnotationKind_Custom extends AnnotationKind {
  const AnnotationKind_Custom({required this.name, required this.value})
      : super._();

  final String name;
  final String value;

  /// Create a copy of AnnotationKind
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $AnnotationKind_CustomCopyWith<AnnotationKind_Custom> get copyWith =>
      _$AnnotationKind_CustomCopyWithImpl<AnnotationKind_Custom>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is AnnotationKind_Custom &&
            (identical(other.name, name) || other.name == name) &&
            (identical(other.value, value) || other.value == value));
  }

  @override
  int get hashCode => Object.hash(runtimeType, name, value);

  @override
  String toString() {
    return 'AnnotationKind.custom(name: $name, value: $value)';
  }
}

/// @nodoc
abstract mixin class $AnnotationKind_CustomCopyWith<$Res>
    implements $AnnotationKindCopyWith<$Res> {
  factory $AnnotationKind_CustomCopyWith(AnnotationKind_Custom value,
          $Res Function(AnnotationKind_Custom) _then) =
      _$AnnotationKind_CustomCopyWithImpl;
  @useResult
  $Res call({String name, String value});
}

/// @nodoc
class _$AnnotationKind_CustomCopyWithImpl<$Res>
    implements $AnnotationKind_CustomCopyWith<$Res> {
  _$AnnotationKind_CustomCopyWithImpl(this._self, this._then);

  final AnnotationKind_Custom _self;
  final $Res Function(AnnotationKind_Custom) _then;

  /// Create a copy of AnnotationKind
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? name = null,
    Object? value = null,
  }) {
    return _then(AnnotationKind_Custom(
      name: null == name
          ? _self.name
          : name // ignore: cast_nullable_to_non_nullable
              as String,
      value: null == value
          ? _self.value
          : value // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc
mixin _$ChunkSizing {
  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is ChunkSizing);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'ChunkSizing()';
  }
}

/// @nodoc
class $ChunkSizingCopyWith<$Res> {
  $ChunkSizingCopyWith(ChunkSizing _, $Res Function(ChunkSizing) __);
}

/// Adds pattern-matching-related methods to [ChunkSizing].
extension ChunkSizingPatterns on ChunkSizing {
  /// A variant of `map` that fallback to returning `orElse`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ChunkSizing_Characters value)? characters,
    TResult Function(ChunkSizing_Tokenizer value)? tokenizer,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case ChunkSizing_Characters() when characters != null:
        return characters(_that);
      case ChunkSizing_Tokenizer() when tokenizer != null:
        return tokenizer(_that);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// Callbacks receives the raw object, upcasted.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case final Subclass2 value:
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ChunkSizing_Characters value) characters,
    required TResult Function(ChunkSizing_Tokenizer value) tokenizer,
  }) {
    final _that = this;
    switch (_that) {
      case ChunkSizing_Characters():
        return characters(_that);
      case ChunkSizing_Tokenizer():
        return tokenizer(_that);
    }
  }

  /// A variant of `map` that fallback to returning `null`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ChunkSizing_Characters value)? characters,
    TResult? Function(ChunkSizing_Tokenizer value)? tokenizer,
  }) {
    final _that = this;
    switch (_that) {
      case ChunkSizing_Characters() when characters != null:
        return characters(_that);
      case ChunkSizing_Tokenizer() when tokenizer != null:
        return tokenizer(_that);
      case _:
        return null;
    }
  }

  /// A variant of `when` that fallback to an `orElse` callback.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? characters,
    TResult Function(String model, String cacheDir)? tokenizer,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case ChunkSizing_Characters() when characters != null:
        return characters();
      case ChunkSizing_Tokenizer() when tokenizer != null:
        return tokenizer(_that.model, _that.cacheDir);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// As opposed to `map`, this offers destructuring.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case Subclass2(:final field2):
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() characters,
    required TResult Function(String model, String cacheDir) tokenizer,
  }) {
    final _that = this;
    switch (_that) {
      case ChunkSizing_Characters():
        return characters();
      case ChunkSizing_Tokenizer():
        return tokenizer(_that.model, _that.cacheDir);
    }
  }

  /// A variant of `when` that fallback to returning `null`
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? characters,
    TResult? Function(String model, String cacheDir)? tokenizer,
  }) {
    final _that = this;
    switch (_that) {
      case ChunkSizing_Characters() when characters != null:
        return characters();
      case ChunkSizing_Tokenizer() when tokenizer != null:
        return tokenizer(_that.model, _that.cacheDir);
      case _:
        return null;
    }
  }
}

/// @nodoc

class ChunkSizing_Characters extends ChunkSizing {
  const ChunkSizing_Characters() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is ChunkSizing_Characters);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'ChunkSizing.characters()';
  }
}

/// @nodoc

class ChunkSizing_Tokenizer extends ChunkSizing {
  const ChunkSizing_Tokenizer({required this.model, required this.cacheDir})
      : super._();

  final String model;
  final String cacheDir;

  /// Create a copy of ChunkSizing
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $ChunkSizing_TokenizerCopyWith<ChunkSizing_Tokenizer> get copyWith =>
      _$ChunkSizing_TokenizerCopyWithImpl<ChunkSizing_Tokenizer>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is ChunkSizing_Tokenizer &&
            (identical(other.model, model) || other.model == model) &&
            (identical(other.cacheDir, cacheDir) ||
                other.cacheDir == cacheDir));
  }

  @override
  int get hashCode => Object.hash(runtimeType, model, cacheDir);

  @override
  String toString() {
    return 'ChunkSizing.tokenizer(model: $model, cacheDir: $cacheDir)';
  }
}

/// @nodoc
abstract mixin class $ChunkSizing_TokenizerCopyWith<$Res>
    implements $ChunkSizingCopyWith<$Res> {
  factory $ChunkSizing_TokenizerCopyWith(ChunkSizing_Tokenizer value,
          $Res Function(ChunkSizing_Tokenizer) _then) =
      _$ChunkSizing_TokenizerCopyWithImpl;
  @useResult
  $Res call({String model, String cacheDir});
}

/// @nodoc
class _$ChunkSizing_TokenizerCopyWithImpl<$Res>
    implements $ChunkSizing_TokenizerCopyWith<$Res> {
  _$ChunkSizing_TokenizerCopyWithImpl(this._self, this._then);

  final ChunkSizing_Tokenizer _self;
  final $Res Function(ChunkSizing_Tokenizer) _then;

  /// Create a copy of ChunkSizing
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? model = null,
    Object? cacheDir = null,
  }) {
    return _then(ChunkSizing_Tokenizer(
      model: null == model
          ? _self.model
          : model // ignore: cast_nullable_to_non_nullable
              as String,
      cacheDir: null == cacheDir
          ? _self.cacheDir
          : cacheDir // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc
mixin _$EmbeddingModelType {
  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is EmbeddingModelType);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'EmbeddingModelType()';
  }
}

/// @nodoc
class $EmbeddingModelTypeCopyWith<$Res> {
  $EmbeddingModelTypeCopyWith(
      EmbeddingModelType _, $Res Function(EmbeddingModelType) __);
}

/// Adds pattern-matching-related methods to [EmbeddingModelType].
extension EmbeddingModelTypePatterns on EmbeddingModelType {
  /// A variant of `map` that fallback to returning `orElse`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(EmbeddingModelType_Preset value)? preset,
    TResult Function(EmbeddingModelType_Custom value)? custom,
    TResult Function(EmbeddingModelType_Llm value)? llm,
    TResult Function(EmbeddingModelType_Plugin value)? plugin,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case EmbeddingModelType_Preset() when preset != null:
        return preset(_that);
      case EmbeddingModelType_Custom() when custom != null:
        return custom(_that);
      case EmbeddingModelType_Llm() when llm != null:
        return llm(_that);
      case EmbeddingModelType_Plugin() when plugin != null:
        return plugin(_that);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// Callbacks receives the raw object, upcasted.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case final Subclass2 value:
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(EmbeddingModelType_Preset value) preset,
    required TResult Function(EmbeddingModelType_Custom value) custom,
    required TResult Function(EmbeddingModelType_Llm value) llm,
    required TResult Function(EmbeddingModelType_Plugin value) plugin,
  }) {
    final _that = this;
    switch (_that) {
      case EmbeddingModelType_Preset():
        return preset(_that);
      case EmbeddingModelType_Custom():
        return custom(_that);
      case EmbeddingModelType_Llm():
        return llm(_that);
      case EmbeddingModelType_Plugin():
        return plugin(_that);
    }
  }

  /// A variant of `map` that fallback to returning `null`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(EmbeddingModelType_Preset value)? preset,
    TResult? Function(EmbeddingModelType_Custom value)? custom,
    TResult? Function(EmbeddingModelType_Llm value)? llm,
    TResult? Function(EmbeddingModelType_Plugin value)? plugin,
  }) {
    final _that = this;
    switch (_that) {
      case EmbeddingModelType_Preset() when preset != null:
        return preset(_that);
      case EmbeddingModelType_Custom() when custom != null:
        return custom(_that);
      case EmbeddingModelType_Llm() when llm != null:
        return llm(_that);
      case EmbeddingModelType_Plugin() when plugin != null:
        return plugin(_that);
      case _:
        return null;
    }
  }

  /// A variant of `when` that fallback to an `orElse` callback.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String name)? preset,
    TResult Function(String modelId, PlatformInt64 dimensions)? custom,
    TResult Function(LlmConfig llm)? llm,
    TResult Function(String name)? plugin,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case EmbeddingModelType_Preset() when preset != null:
        return preset(_that.name);
      case EmbeddingModelType_Custom() when custom != null:
        return custom(_that.modelId, _that.dimensions);
      case EmbeddingModelType_Llm() when llm != null:
        return llm(_that.llm);
      case EmbeddingModelType_Plugin() when plugin != null:
        return plugin(_that.name);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// As opposed to `map`, this offers destructuring.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case Subclass2(:final field2):
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String name) preset,
    required TResult Function(String modelId, PlatformInt64 dimensions) custom,
    required TResult Function(LlmConfig llm) llm,
    required TResult Function(String name) plugin,
  }) {
    final _that = this;
    switch (_that) {
      case EmbeddingModelType_Preset():
        return preset(_that.name);
      case EmbeddingModelType_Custom():
        return custom(_that.modelId, _that.dimensions);
      case EmbeddingModelType_Llm():
        return llm(_that.llm);
      case EmbeddingModelType_Plugin():
        return plugin(_that.name);
    }
  }

  /// A variant of `when` that fallback to returning `null`
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String name)? preset,
    TResult? Function(String modelId, PlatformInt64 dimensions)? custom,
    TResult? Function(LlmConfig llm)? llm,
    TResult? Function(String name)? plugin,
  }) {
    final _that = this;
    switch (_that) {
      case EmbeddingModelType_Preset() when preset != null:
        return preset(_that.name);
      case EmbeddingModelType_Custom() when custom != null:
        return custom(_that.modelId, _that.dimensions);
      case EmbeddingModelType_Llm() when llm != null:
        return llm(_that.llm);
      case EmbeddingModelType_Plugin() when plugin != null:
        return plugin(_that.name);
      case _:
        return null;
    }
  }
}

/// @nodoc

class EmbeddingModelType_Preset extends EmbeddingModelType {
  const EmbeddingModelType_Preset({required this.name}) : super._();

  final String name;

  /// Create a copy of EmbeddingModelType
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $EmbeddingModelType_PresetCopyWith<EmbeddingModelType_Preset> get copyWith =>
      _$EmbeddingModelType_PresetCopyWithImpl<EmbeddingModelType_Preset>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is EmbeddingModelType_Preset &&
            (identical(other.name, name) || other.name == name));
  }

  @override
  int get hashCode => Object.hash(runtimeType, name);

  @override
  String toString() {
    return 'EmbeddingModelType.preset(name: $name)';
  }
}

/// @nodoc
abstract mixin class $EmbeddingModelType_PresetCopyWith<$Res>
    implements $EmbeddingModelTypeCopyWith<$Res> {
  factory $EmbeddingModelType_PresetCopyWith(EmbeddingModelType_Preset value,
          $Res Function(EmbeddingModelType_Preset) _then) =
      _$EmbeddingModelType_PresetCopyWithImpl;
  @useResult
  $Res call({String name});
}

/// @nodoc
class _$EmbeddingModelType_PresetCopyWithImpl<$Res>
    implements $EmbeddingModelType_PresetCopyWith<$Res> {
  _$EmbeddingModelType_PresetCopyWithImpl(this._self, this._then);

  final EmbeddingModelType_Preset _self;
  final $Res Function(EmbeddingModelType_Preset) _then;

  /// Create a copy of EmbeddingModelType
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? name = null,
  }) {
    return _then(EmbeddingModelType_Preset(
      name: null == name
          ? _self.name
          : name // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class EmbeddingModelType_Custom extends EmbeddingModelType {
  const EmbeddingModelType_Custom(
      {required this.modelId, required this.dimensions})
      : super._();

  final String modelId;
  final PlatformInt64 dimensions;

  /// Create a copy of EmbeddingModelType
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $EmbeddingModelType_CustomCopyWith<EmbeddingModelType_Custom> get copyWith =>
      _$EmbeddingModelType_CustomCopyWithImpl<EmbeddingModelType_Custom>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is EmbeddingModelType_Custom &&
            (identical(other.modelId, modelId) || other.modelId == modelId) &&
            (identical(other.dimensions, dimensions) ||
                other.dimensions == dimensions));
  }

  @override
  int get hashCode => Object.hash(runtimeType, modelId, dimensions);

  @override
  String toString() {
    return 'EmbeddingModelType.custom(modelId: $modelId, dimensions: $dimensions)';
  }
}

/// @nodoc
abstract mixin class $EmbeddingModelType_CustomCopyWith<$Res>
    implements $EmbeddingModelTypeCopyWith<$Res> {
  factory $EmbeddingModelType_CustomCopyWith(EmbeddingModelType_Custom value,
          $Res Function(EmbeddingModelType_Custom) _then) =
      _$EmbeddingModelType_CustomCopyWithImpl;
  @useResult
  $Res call({String modelId, PlatformInt64 dimensions});
}

/// @nodoc
class _$EmbeddingModelType_CustomCopyWithImpl<$Res>
    implements $EmbeddingModelType_CustomCopyWith<$Res> {
  _$EmbeddingModelType_CustomCopyWithImpl(this._self, this._then);

  final EmbeddingModelType_Custom _self;
  final $Res Function(EmbeddingModelType_Custom) _then;

  /// Create a copy of EmbeddingModelType
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? modelId = null,
    Object? dimensions = null,
  }) {
    return _then(EmbeddingModelType_Custom(
      modelId: null == modelId
          ? _self.modelId
          : modelId // ignore: cast_nullable_to_non_nullable
              as String,
      dimensions: null == dimensions
          ? _self.dimensions
          : dimensions // ignore: cast_nullable_to_non_nullable
              as PlatformInt64,
    ));
  }
}

/// @nodoc

class EmbeddingModelType_Llm extends EmbeddingModelType {
  const EmbeddingModelType_Llm({required this.llm}) : super._();

  final LlmConfig llm;

  /// Create a copy of EmbeddingModelType
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $EmbeddingModelType_LlmCopyWith<EmbeddingModelType_Llm> get copyWith =>
      _$EmbeddingModelType_LlmCopyWithImpl<EmbeddingModelType_Llm>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is EmbeddingModelType_Llm &&
            (identical(other.llm, llm) || other.llm == llm));
  }

  @override
  int get hashCode => Object.hash(runtimeType, llm);

  @override
  String toString() {
    return 'EmbeddingModelType.llm(llm: $llm)';
  }
}

/// @nodoc
abstract mixin class $EmbeddingModelType_LlmCopyWith<$Res>
    implements $EmbeddingModelTypeCopyWith<$Res> {
  factory $EmbeddingModelType_LlmCopyWith(EmbeddingModelType_Llm value,
          $Res Function(EmbeddingModelType_Llm) _then) =
      _$EmbeddingModelType_LlmCopyWithImpl;
  @useResult
  $Res call({LlmConfig llm});
}

/// @nodoc
class _$EmbeddingModelType_LlmCopyWithImpl<$Res>
    implements $EmbeddingModelType_LlmCopyWith<$Res> {
  _$EmbeddingModelType_LlmCopyWithImpl(this._self, this._then);

  final EmbeddingModelType_Llm _self;
  final $Res Function(EmbeddingModelType_Llm) _then;

  /// Create a copy of EmbeddingModelType
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? llm = null,
  }) {
    return _then(EmbeddingModelType_Llm(
      llm: null == llm
          ? _self.llm
          : llm // ignore: cast_nullable_to_non_nullable
              as LlmConfig,
    ));
  }
}

/// @nodoc

class EmbeddingModelType_Plugin extends EmbeddingModelType {
  const EmbeddingModelType_Plugin({required this.name}) : super._();

  final String name;

  /// Create a copy of EmbeddingModelType
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $EmbeddingModelType_PluginCopyWith<EmbeddingModelType_Plugin> get copyWith =>
      _$EmbeddingModelType_PluginCopyWithImpl<EmbeddingModelType_Plugin>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is EmbeddingModelType_Plugin &&
            (identical(other.name, name) || other.name == name));
  }

  @override
  int get hashCode => Object.hash(runtimeType, name);

  @override
  String toString() {
    return 'EmbeddingModelType.plugin(name: $name)';
  }
}

/// @nodoc
abstract mixin class $EmbeddingModelType_PluginCopyWith<$Res>
    implements $EmbeddingModelTypeCopyWith<$Res> {
  factory $EmbeddingModelType_PluginCopyWith(EmbeddingModelType_Plugin value,
          $Res Function(EmbeddingModelType_Plugin) _then) =
      _$EmbeddingModelType_PluginCopyWithImpl;
  @useResult
  $Res call({String name});
}

/// @nodoc
class _$EmbeddingModelType_PluginCopyWithImpl<$Res>
    implements $EmbeddingModelType_PluginCopyWith<$Res> {
  _$EmbeddingModelType_PluginCopyWithImpl(this._self, this._then);

  final EmbeddingModelType_Plugin _self;
  final $Res Function(EmbeddingModelType_Plugin) _then;

  /// Create a copy of EmbeddingModelType
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? name = null,
  }) {
    return _then(EmbeddingModelType_Plugin(
      name: null == name
          ? _self.name
          : name // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc
mixin _$FormatMetadata {
  Object get field0;

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata &&
            const DeepCollectionEquality().equals(other.field0, field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(field0));

  @override
  String toString() {
    return 'FormatMetadata(field0: $field0)';
  }
}

/// @nodoc
class $FormatMetadataCopyWith<$Res> {
  $FormatMetadataCopyWith(FormatMetadata _, $Res Function(FormatMetadata) __);
}

/// Adds pattern-matching-related methods to [FormatMetadata].
extension FormatMetadataPatterns on FormatMetadata {
  /// A variant of `map` that fallback to returning `orElse`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(FormatMetadata_Pdf value)? pdf,
    TResult Function(FormatMetadata_Docx value)? docx,
    TResult Function(FormatMetadata_Excel value)? excel,
    TResult Function(FormatMetadata_Email value)? email,
    TResult Function(FormatMetadata_Pptx value)? pptx,
    TResult Function(FormatMetadata_Archive value)? archive,
    TResult Function(FormatMetadata_Image value)? image,
    TResult Function(FormatMetadata_Xml value)? xml,
    TResult Function(FormatMetadata_Text value)? text,
    TResult Function(FormatMetadata_Html value)? html,
    TResult Function(FormatMetadata_Ocr value)? ocr,
    TResult Function(FormatMetadata_Csv value)? csv,
    TResult Function(FormatMetadata_Bibtex value)? bibtex,
    TResult Function(FormatMetadata_Citation value)? citation,
    TResult Function(FormatMetadata_FictionBook value)? fictionBook,
    TResult Function(FormatMetadata_Dbf value)? dbf,
    TResult Function(FormatMetadata_Jats value)? jats,
    TResult Function(FormatMetadata_Epub value)? epub,
    TResult Function(FormatMetadata_Pst value)? pst,
    TResult Function(FormatMetadata_Code value)? code,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case FormatMetadata_Pdf() when pdf != null:
        return pdf(_that);
      case FormatMetadata_Docx() when docx != null:
        return docx(_that);
      case FormatMetadata_Excel() when excel != null:
        return excel(_that);
      case FormatMetadata_Email() when email != null:
        return email(_that);
      case FormatMetadata_Pptx() when pptx != null:
        return pptx(_that);
      case FormatMetadata_Archive() when archive != null:
        return archive(_that);
      case FormatMetadata_Image() when image != null:
        return image(_that);
      case FormatMetadata_Xml() when xml != null:
        return xml(_that);
      case FormatMetadata_Text() when text != null:
        return text(_that);
      case FormatMetadata_Html() when html != null:
        return html(_that);
      case FormatMetadata_Ocr() when ocr != null:
        return ocr(_that);
      case FormatMetadata_Csv() when csv != null:
        return csv(_that);
      case FormatMetadata_Bibtex() when bibtex != null:
        return bibtex(_that);
      case FormatMetadata_Citation() when citation != null:
        return citation(_that);
      case FormatMetadata_FictionBook() when fictionBook != null:
        return fictionBook(_that);
      case FormatMetadata_Dbf() when dbf != null:
        return dbf(_that);
      case FormatMetadata_Jats() when jats != null:
        return jats(_that);
      case FormatMetadata_Epub() when epub != null:
        return epub(_that);
      case FormatMetadata_Pst() when pst != null:
        return pst(_that);
      case FormatMetadata_Code() when code != null:
        return code(_that);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// Callbacks receives the raw object, upcasted.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case final Subclass2 value:
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(FormatMetadata_Pdf value) pdf,
    required TResult Function(FormatMetadata_Docx value) docx,
    required TResult Function(FormatMetadata_Excel value) excel,
    required TResult Function(FormatMetadata_Email value) email,
    required TResult Function(FormatMetadata_Pptx value) pptx,
    required TResult Function(FormatMetadata_Archive value) archive,
    required TResult Function(FormatMetadata_Image value) image,
    required TResult Function(FormatMetadata_Xml value) xml,
    required TResult Function(FormatMetadata_Text value) text,
    required TResult Function(FormatMetadata_Html value) html,
    required TResult Function(FormatMetadata_Ocr value) ocr,
    required TResult Function(FormatMetadata_Csv value) csv,
    required TResult Function(FormatMetadata_Bibtex value) bibtex,
    required TResult Function(FormatMetadata_Citation value) citation,
    required TResult Function(FormatMetadata_FictionBook value) fictionBook,
    required TResult Function(FormatMetadata_Dbf value) dbf,
    required TResult Function(FormatMetadata_Jats value) jats,
    required TResult Function(FormatMetadata_Epub value) epub,
    required TResult Function(FormatMetadata_Pst value) pst,
    required TResult Function(FormatMetadata_Code value) code,
  }) {
    final _that = this;
    switch (_that) {
      case FormatMetadata_Pdf():
        return pdf(_that);
      case FormatMetadata_Docx():
        return docx(_that);
      case FormatMetadata_Excel():
        return excel(_that);
      case FormatMetadata_Email():
        return email(_that);
      case FormatMetadata_Pptx():
        return pptx(_that);
      case FormatMetadata_Archive():
        return archive(_that);
      case FormatMetadata_Image():
        return image(_that);
      case FormatMetadata_Xml():
        return xml(_that);
      case FormatMetadata_Text():
        return text(_that);
      case FormatMetadata_Html():
        return html(_that);
      case FormatMetadata_Ocr():
        return ocr(_that);
      case FormatMetadata_Csv():
        return csv(_that);
      case FormatMetadata_Bibtex():
        return bibtex(_that);
      case FormatMetadata_Citation():
        return citation(_that);
      case FormatMetadata_FictionBook():
        return fictionBook(_that);
      case FormatMetadata_Dbf():
        return dbf(_that);
      case FormatMetadata_Jats():
        return jats(_that);
      case FormatMetadata_Epub():
        return epub(_that);
      case FormatMetadata_Pst():
        return pst(_that);
      case FormatMetadata_Code():
        return code(_that);
    }
  }

  /// A variant of `map` that fallback to returning `null`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(FormatMetadata_Pdf value)? pdf,
    TResult? Function(FormatMetadata_Docx value)? docx,
    TResult? Function(FormatMetadata_Excel value)? excel,
    TResult? Function(FormatMetadata_Email value)? email,
    TResult? Function(FormatMetadata_Pptx value)? pptx,
    TResult? Function(FormatMetadata_Archive value)? archive,
    TResult? Function(FormatMetadata_Image value)? image,
    TResult? Function(FormatMetadata_Xml value)? xml,
    TResult? Function(FormatMetadata_Text value)? text,
    TResult? Function(FormatMetadata_Html value)? html,
    TResult? Function(FormatMetadata_Ocr value)? ocr,
    TResult? Function(FormatMetadata_Csv value)? csv,
    TResult? Function(FormatMetadata_Bibtex value)? bibtex,
    TResult? Function(FormatMetadata_Citation value)? citation,
    TResult? Function(FormatMetadata_FictionBook value)? fictionBook,
    TResult? Function(FormatMetadata_Dbf value)? dbf,
    TResult? Function(FormatMetadata_Jats value)? jats,
    TResult? Function(FormatMetadata_Epub value)? epub,
    TResult? Function(FormatMetadata_Pst value)? pst,
    TResult? Function(FormatMetadata_Code value)? code,
  }) {
    final _that = this;
    switch (_that) {
      case FormatMetadata_Pdf() when pdf != null:
        return pdf(_that);
      case FormatMetadata_Docx() when docx != null:
        return docx(_that);
      case FormatMetadata_Excel() when excel != null:
        return excel(_that);
      case FormatMetadata_Email() when email != null:
        return email(_that);
      case FormatMetadata_Pptx() when pptx != null:
        return pptx(_that);
      case FormatMetadata_Archive() when archive != null:
        return archive(_that);
      case FormatMetadata_Image() when image != null:
        return image(_that);
      case FormatMetadata_Xml() when xml != null:
        return xml(_that);
      case FormatMetadata_Text() when text != null:
        return text(_that);
      case FormatMetadata_Html() when html != null:
        return html(_that);
      case FormatMetadata_Ocr() when ocr != null:
        return ocr(_that);
      case FormatMetadata_Csv() when csv != null:
        return csv(_that);
      case FormatMetadata_Bibtex() when bibtex != null:
        return bibtex(_that);
      case FormatMetadata_Citation() when citation != null:
        return citation(_that);
      case FormatMetadata_FictionBook() when fictionBook != null:
        return fictionBook(_that);
      case FormatMetadata_Dbf() when dbf != null:
        return dbf(_that);
      case FormatMetadata_Jats() when jats != null:
        return jats(_that);
      case FormatMetadata_Epub() when epub != null:
        return epub(_that);
      case FormatMetadata_Pst() when pst != null:
        return pst(_that);
      case FormatMetadata_Code() when code != null:
        return code(_that);
      case _:
        return null;
    }
  }

  /// A variant of `when` that fallback to an `orElse` callback.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(PdfMetadata field0)? pdf,
    TResult Function(DocxMetadata field0)? docx,
    TResult Function(ExcelMetadata field0)? excel,
    TResult Function(EmailMetadata field0)? email,
    TResult Function(PptxMetadata field0)? pptx,
    TResult Function(ArchiveMetadata field0)? archive,
    TResult Function(ImageMetadata field0)? image,
    TResult Function(XmlMetadata field0)? xml,
    TResult Function(TextMetadata field0)? text,
    TResult Function(HtmlMetadata field0)? html,
    TResult Function(OcrMetadata field0)? ocr,
    TResult Function(CsvMetadata field0)? csv,
    TResult Function(BibtexMetadata field0)? bibtex,
    TResult Function(CitationMetadata field0)? citation,
    TResult Function(FictionBookMetadata field0)? fictionBook,
    TResult Function(DbfMetadata field0)? dbf,
    TResult Function(JatsMetadata field0)? jats,
    TResult Function(EpubMetadata field0)? epub,
    TResult Function(PstMetadata field0)? pst,
    TResult Function(String field0)? code,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case FormatMetadata_Pdf() when pdf != null:
        return pdf(_that.field0);
      case FormatMetadata_Docx() when docx != null:
        return docx(_that.field0);
      case FormatMetadata_Excel() when excel != null:
        return excel(_that.field0);
      case FormatMetadata_Email() when email != null:
        return email(_that.field0);
      case FormatMetadata_Pptx() when pptx != null:
        return pptx(_that.field0);
      case FormatMetadata_Archive() when archive != null:
        return archive(_that.field0);
      case FormatMetadata_Image() when image != null:
        return image(_that.field0);
      case FormatMetadata_Xml() when xml != null:
        return xml(_that.field0);
      case FormatMetadata_Text() when text != null:
        return text(_that.field0);
      case FormatMetadata_Html() when html != null:
        return html(_that.field0);
      case FormatMetadata_Ocr() when ocr != null:
        return ocr(_that.field0);
      case FormatMetadata_Csv() when csv != null:
        return csv(_that.field0);
      case FormatMetadata_Bibtex() when bibtex != null:
        return bibtex(_that.field0);
      case FormatMetadata_Citation() when citation != null:
        return citation(_that.field0);
      case FormatMetadata_FictionBook() when fictionBook != null:
        return fictionBook(_that.field0);
      case FormatMetadata_Dbf() when dbf != null:
        return dbf(_that.field0);
      case FormatMetadata_Jats() when jats != null:
        return jats(_that.field0);
      case FormatMetadata_Epub() when epub != null:
        return epub(_that.field0);
      case FormatMetadata_Pst() when pst != null:
        return pst(_that.field0);
      case FormatMetadata_Code() when code != null:
        return code(_that.field0);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// As opposed to `map`, this offers destructuring.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case Subclass2(:final field2):
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(PdfMetadata field0) pdf,
    required TResult Function(DocxMetadata field0) docx,
    required TResult Function(ExcelMetadata field0) excel,
    required TResult Function(EmailMetadata field0) email,
    required TResult Function(PptxMetadata field0) pptx,
    required TResult Function(ArchiveMetadata field0) archive,
    required TResult Function(ImageMetadata field0) image,
    required TResult Function(XmlMetadata field0) xml,
    required TResult Function(TextMetadata field0) text,
    required TResult Function(HtmlMetadata field0) html,
    required TResult Function(OcrMetadata field0) ocr,
    required TResult Function(CsvMetadata field0) csv,
    required TResult Function(BibtexMetadata field0) bibtex,
    required TResult Function(CitationMetadata field0) citation,
    required TResult Function(FictionBookMetadata field0) fictionBook,
    required TResult Function(DbfMetadata field0) dbf,
    required TResult Function(JatsMetadata field0) jats,
    required TResult Function(EpubMetadata field0) epub,
    required TResult Function(PstMetadata field0) pst,
    required TResult Function(String field0) code,
  }) {
    final _that = this;
    switch (_that) {
      case FormatMetadata_Pdf():
        return pdf(_that.field0);
      case FormatMetadata_Docx():
        return docx(_that.field0);
      case FormatMetadata_Excel():
        return excel(_that.field0);
      case FormatMetadata_Email():
        return email(_that.field0);
      case FormatMetadata_Pptx():
        return pptx(_that.field0);
      case FormatMetadata_Archive():
        return archive(_that.field0);
      case FormatMetadata_Image():
        return image(_that.field0);
      case FormatMetadata_Xml():
        return xml(_that.field0);
      case FormatMetadata_Text():
        return text(_that.field0);
      case FormatMetadata_Html():
        return html(_that.field0);
      case FormatMetadata_Ocr():
        return ocr(_that.field0);
      case FormatMetadata_Csv():
        return csv(_that.field0);
      case FormatMetadata_Bibtex():
        return bibtex(_that.field0);
      case FormatMetadata_Citation():
        return citation(_that.field0);
      case FormatMetadata_FictionBook():
        return fictionBook(_that.field0);
      case FormatMetadata_Dbf():
        return dbf(_that.field0);
      case FormatMetadata_Jats():
        return jats(_that.field0);
      case FormatMetadata_Epub():
        return epub(_that.field0);
      case FormatMetadata_Pst():
        return pst(_that.field0);
      case FormatMetadata_Code():
        return code(_that.field0);
    }
  }

  /// A variant of `when` that fallback to returning `null`
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(PdfMetadata field0)? pdf,
    TResult? Function(DocxMetadata field0)? docx,
    TResult? Function(ExcelMetadata field0)? excel,
    TResult? Function(EmailMetadata field0)? email,
    TResult? Function(PptxMetadata field0)? pptx,
    TResult? Function(ArchiveMetadata field0)? archive,
    TResult? Function(ImageMetadata field0)? image,
    TResult? Function(XmlMetadata field0)? xml,
    TResult? Function(TextMetadata field0)? text,
    TResult? Function(HtmlMetadata field0)? html,
    TResult? Function(OcrMetadata field0)? ocr,
    TResult? Function(CsvMetadata field0)? csv,
    TResult? Function(BibtexMetadata field0)? bibtex,
    TResult? Function(CitationMetadata field0)? citation,
    TResult? Function(FictionBookMetadata field0)? fictionBook,
    TResult? Function(DbfMetadata field0)? dbf,
    TResult? Function(JatsMetadata field0)? jats,
    TResult? Function(EpubMetadata field0)? epub,
    TResult? Function(PstMetadata field0)? pst,
    TResult? Function(String field0)? code,
  }) {
    final _that = this;
    switch (_that) {
      case FormatMetadata_Pdf() when pdf != null:
        return pdf(_that.field0);
      case FormatMetadata_Docx() when docx != null:
        return docx(_that.field0);
      case FormatMetadata_Excel() when excel != null:
        return excel(_that.field0);
      case FormatMetadata_Email() when email != null:
        return email(_that.field0);
      case FormatMetadata_Pptx() when pptx != null:
        return pptx(_that.field0);
      case FormatMetadata_Archive() when archive != null:
        return archive(_that.field0);
      case FormatMetadata_Image() when image != null:
        return image(_that.field0);
      case FormatMetadata_Xml() when xml != null:
        return xml(_that.field0);
      case FormatMetadata_Text() when text != null:
        return text(_that.field0);
      case FormatMetadata_Html() when html != null:
        return html(_that.field0);
      case FormatMetadata_Ocr() when ocr != null:
        return ocr(_that.field0);
      case FormatMetadata_Csv() when csv != null:
        return csv(_that.field0);
      case FormatMetadata_Bibtex() when bibtex != null:
        return bibtex(_that.field0);
      case FormatMetadata_Citation() when citation != null:
        return citation(_that.field0);
      case FormatMetadata_FictionBook() when fictionBook != null:
        return fictionBook(_that.field0);
      case FormatMetadata_Dbf() when dbf != null:
        return dbf(_that.field0);
      case FormatMetadata_Jats() when jats != null:
        return jats(_that.field0);
      case FormatMetadata_Epub() when epub != null:
        return epub(_that.field0);
      case FormatMetadata_Pst() when pst != null:
        return pst(_that.field0);
      case FormatMetadata_Code() when code != null:
        return code(_that.field0);
      case _:
        return null;
    }
  }
}

/// @nodoc

class FormatMetadata_Pdf extends FormatMetadata {
  const FormatMetadata_Pdf({required this.field0}) : super._();

  @override
  final PdfMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_PdfCopyWith<FormatMetadata_Pdf> get copyWith =>
      _$FormatMetadata_PdfCopyWithImpl<FormatMetadata_Pdf>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Pdf &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.pdf(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_PdfCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_PdfCopyWith(
          FormatMetadata_Pdf value, $Res Function(FormatMetadata_Pdf) _then) =
      _$FormatMetadata_PdfCopyWithImpl;
  @useResult
  $Res call({PdfMetadata field0});
}

/// @nodoc
class _$FormatMetadata_PdfCopyWithImpl<$Res>
    implements $FormatMetadata_PdfCopyWith<$Res> {
  _$FormatMetadata_PdfCopyWithImpl(this._self, this._then);

  final FormatMetadata_Pdf _self;
  final $Res Function(FormatMetadata_Pdf) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Pdf(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as PdfMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Docx extends FormatMetadata {
  const FormatMetadata_Docx({required this.field0}) : super._();

  @override
  final DocxMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_DocxCopyWith<FormatMetadata_Docx> get copyWith =>
      _$FormatMetadata_DocxCopyWithImpl<FormatMetadata_Docx>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Docx &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.docx(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_DocxCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_DocxCopyWith(
          FormatMetadata_Docx value, $Res Function(FormatMetadata_Docx) _then) =
      _$FormatMetadata_DocxCopyWithImpl;
  @useResult
  $Res call({DocxMetadata field0});
}

/// @nodoc
class _$FormatMetadata_DocxCopyWithImpl<$Res>
    implements $FormatMetadata_DocxCopyWith<$Res> {
  _$FormatMetadata_DocxCopyWithImpl(this._self, this._then);

  final FormatMetadata_Docx _self;
  final $Res Function(FormatMetadata_Docx) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Docx(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as DocxMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Excel extends FormatMetadata {
  const FormatMetadata_Excel({required this.field0}) : super._();

  @override
  final ExcelMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_ExcelCopyWith<FormatMetadata_Excel> get copyWith =>
      _$FormatMetadata_ExcelCopyWithImpl<FormatMetadata_Excel>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Excel &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.excel(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_ExcelCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_ExcelCopyWith(FormatMetadata_Excel value,
          $Res Function(FormatMetadata_Excel) _then) =
      _$FormatMetadata_ExcelCopyWithImpl;
  @useResult
  $Res call({ExcelMetadata field0});
}

/// @nodoc
class _$FormatMetadata_ExcelCopyWithImpl<$Res>
    implements $FormatMetadata_ExcelCopyWith<$Res> {
  _$FormatMetadata_ExcelCopyWithImpl(this._self, this._then);

  final FormatMetadata_Excel _self;
  final $Res Function(FormatMetadata_Excel) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Excel(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as ExcelMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Email extends FormatMetadata {
  const FormatMetadata_Email({required this.field0}) : super._();

  @override
  final EmailMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_EmailCopyWith<FormatMetadata_Email> get copyWith =>
      _$FormatMetadata_EmailCopyWithImpl<FormatMetadata_Email>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Email &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.email(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_EmailCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_EmailCopyWith(FormatMetadata_Email value,
          $Res Function(FormatMetadata_Email) _then) =
      _$FormatMetadata_EmailCopyWithImpl;
  @useResult
  $Res call({EmailMetadata field0});
}

/// @nodoc
class _$FormatMetadata_EmailCopyWithImpl<$Res>
    implements $FormatMetadata_EmailCopyWith<$Res> {
  _$FormatMetadata_EmailCopyWithImpl(this._self, this._then);

  final FormatMetadata_Email _self;
  final $Res Function(FormatMetadata_Email) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Email(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as EmailMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Pptx extends FormatMetadata {
  const FormatMetadata_Pptx({required this.field0}) : super._();

  @override
  final PptxMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_PptxCopyWith<FormatMetadata_Pptx> get copyWith =>
      _$FormatMetadata_PptxCopyWithImpl<FormatMetadata_Pptx>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Pptx &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.pptx(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_PptxCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_PptxCopyWith(
          FormatMetadata_Pptx value, $Res Function(FormatMetadata_Pptx) _then) =
      _$FormatMetadata_PptxCopyWithImpl;
  @useResult
  $Res call({PptxMetadata field0});
}

/// @nodoc
class _$FormatMetadata_PptxCopyWithImpl<$Res>
    implements $FormatMetadata_PptxCopyWith<$Res> {
  _$FormatMetadata_PptxCopyWithImpl(this._self, this._then);

  final FormatMetadata_Pptx _self;
  final $Res Function(FormatMetadata_Pptx) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Pptx(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as PptxMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Archive extends FormatMetadata {
  const FormatMetadata_Archive({required this.field0}) : super._();

  @override
  final ArchiveMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_ArchiveCopyWith<FormatMetadata_Archive> get copyWith =>
      _$FormatMetadata_ArchiveCopyWithImpl<FormatMetadata_Archive>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Archive &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.archive(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_ArchiveCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_ArchiveCopyWith(FormatMetadata_Archive value,
          $Res Function(FormatMetadata_Archive) _then) =
      _$FormatMetadata_ArchiveCopyWithImpl;
  @useResult
  $Res call({ArchiveMetadata field0});
}

/// @nodoc
class _$FormatMetadata_ArchiveCopyWithImpl<$Res>
    implements $FormatMetadata_ArchiveCopyWith<$Res> {
  _$FormatMetadata_ArchiveCopyWithImpl(this._self, this._then);

  final FormatMetadata_Archive _self;
  final $Res Function(FormatMetadata_Archive) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Archive(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as ArchiveMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Image extends FormatMetadata {
  const FormatMetadata_Image({required this.field0}) : super._();

  @override
  final ImageMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_ImageCopyWith<FormatMetadata_Image> get copyWith =>
      _$FormatMetadata_ImageCopyWithImpl<FormatMetadata_Image>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Image &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.image(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_ImageCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_ImageCopyWith(FormatMetadata_Image value,
          $Res Function(FormatMetadata_Image) _then) =
      _$FormatMetadata_ImageCopyWithImpl;
  @useResult
  $Res call({ImageMetadata field0});
}

/// @nodoc
class _$FormatMetadata_ImageCopyWithImpl<$Res>
    implements $FormatMetadata_ImageCopyWith<$Res> {
  _$FormatMetadata_ImageCopyWithImpl(this._self, this._then);

  final FormatMetadata_Image _self;
  final $Res Function(FormatMetadata_Image) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Image(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as ImageMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Xml extends FormatMetadata {
  const FormatMetadata_Xml({required this.field0}) : super._();

  @override
  final XmlMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_XmlCopyWith<FormatMetadata_Xml> get copyWith =>
      _$FormatMetadata_XmlCopyWithImpl<FormatMetadata_Xml>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Xml &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.xml(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_XmlCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_XmlCopyWith(
          FormatMetadata_Xml value, $Res Function(FormatMetadata_Xml) _then) =
      _$FormatMetadata_XmlCopyWithImpl;
  @useResult
  $Res call({XmlMetadata field0});
}

/// @nodoc
class _$FormatMetadata_XmlCopyWithImpl<$Res>
    implements $FormatMetadata_XmlCopyWith<$Res> {
  _$FormatMetadata_XmlCopyWithImpl(this._self, this._then);

  final FormatMetadata_Xml _self;
  final $Res Function(FormatMetadata_Xml) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Xml(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as XmlMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Text extends FormatMetadata {
  const FormatMetadata_Text({required this.field0}) : super._();

  @override
  final TextMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_TextCopyWith<FormatMetadata_Text> get copyWith =>
      _$FormatMetadata_TextCopyWithImpl<FormatMetadata_Text>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Text &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.text(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_TextCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_TextCopyWith(
          FormatMetadata_Text value, $Res Function(FormatMetadata_Text) _then) =
      _$FormatMetadata_TextCopyWithImpl;
  @useResult
  $Res call({TextMetadata field0});
}

/// @nodoc
class _$FormatMetadata_TextCopyWithImpl<$Res>
    implements $FormatMetadata_TextCopyWith<$Res> {
  _$FormatMetadata_TextCopyWithImpl(this._self, this._then);

  final FormatMetadata_Text _self;
  final $Res Function(FormatMetadata_Text) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Text(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as TextMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Html extends FormatMetadata {
  const FormatMetadata_Html({required this.field0}) : super._();

  @override
  final HtmlMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_HtmlCopyWith<FormatMetadata_Html> get copyWith =>
      _$FormatMetadata_HtmlCopyWithImpl<FormatMetadata_Html>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Html &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.html(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_HtmlCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_HtmlCopyWith(
          FormatMetadata_Html value, $Res Function(FormatMetadata_Html) _then) =
      _$FormatMetadata_HtmlCopyWithImpl;
  @useResult
  $Res call({HtmlMetadata field0});
}

/// @nodoc
class _$FormatMetadata_HtmlCopyWithImpl<$Res>
    implements $FormatMetadata_HtmlCopyWith<$Res> {
  _$FormatMetadata_HtmlCopyWithImpl(this._self, this._then);

  final FormatMetadata_Html _self;
  final $Res Function(FormatMetadata_Html) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Html(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as HtmlMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Ocr extends FormatMetadata {
  const FormatMetadata_Ocr({required this.field0}) : super._();

  @override
  final OcrMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_OcrCopyWith<FormatMetadata_Ocr> get copyWith =>
      _$FormatMetadata_OcrCopyWithImpl<FormatMetadata_Ocr>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Ocr &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.ocr(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_OcrCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_OcrCopyWith(
          FormatMetadata_Ocr value, $Res Function(FormatMetadata_Ocr) _then) =
      _$FormatMetadata_OcrCopyWithImpl;
  @useResult
  $Res call({OcrMetadata field0});
}

/// @nodoc
class _$FormatMetadata_OcrCopyWithImpl<$Res>
    implements $FormatMetadata_OcrCopyWith<$Res> {
  _$FormatMetadata_OcrCopyWithImpl(this._self, this._then);

  final FormatMetadata_Ocr _self;
  final $Res Function(FormatMetadata_Ocr) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Ocr(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as OcrMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Csv extends FormatMetadata {
  const FormatMetadata_Csv({required this.field0}) : super._();

  @override
  final CsvMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_CsvCopyWith<FormatMetadata_Csv> get copyWith =>
      _$FormatMetadata_CsvCopyWithImpl<FormatMetadata_Csv>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Csv &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.csv(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_CsvCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_CsvCopyWith(
          FormatMetadata_Csv value, $Res Function(FormatMetadata_Csv) _then) =
      _$FormatMetadata_CsvCopyWithImpl;
  @useResult
  $Res call({CsvMetadata field0});
}

/// @nodoc
class _$FormatMetadata_CsvCopyWithImpl<$Res>
    implements $FormatMetadata_CsvCopyWith<$Res> {
  _$FormatMetadata_CsvCopyWithImpl(this._self, this._then);

  final FormatMetadata_Csv _self;
  final $Res Function(FormatMetadata_Csv) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Csv(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as CsvMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Bibtex extends FormatMetadata {
  const FormatMetadata_Bibtex({required this.field0}) : super._();

  @override
  final BibtexMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_BibtexCopyWith<FormatMetadata_Bibtex> get copyWith =>
      _$FormatMetadata_BibtexCopyWithImpl<FormatMetadata_Bibtex>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Bibtex &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.bibtex(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_BibtexCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_BibtexCopyWith(FormatMetadata_Bibtex value,
          $Res Function(FormatMetadata_Bibtex) _then) =
      _$FormatMetadata_BibtexCopyWithImpl;
  @useResult
  $Res call({BibtexMetadata field0});
}

/// @nodoc
class _$FormatMetadata_BibtexCopyWithImpl<$Res>
    implements $FormatMetadata_BibtexCopyWith<$Res> {
  _$FormatMetadata_BibtexCopyWithImpl(this._self, this._then);

  final FormatMetadata_Bibtex _self;
  final $Res Function(FormatMetadata_Bibtex) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Bibtex(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as BibtexMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Citation extends FormatMetadata {
  const FormatMetadata_Citation({required this.field0}) : super._();

  @override
  final CitationMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_CitationCopyWith<FormatMetadata_Citation> get copyWith =>
      _$FormatMetadata_CitationCopyWithImpl<FormatMetadata_Citation>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Citation &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.citation(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_CitationCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_CitationCopyWith(FormatMetadata_Citation value,
          $Res Function(FormatMetadata_Citation) _then) =
      _$FormatMetadata_CitationCopyWithImpl;
  @useResult
  $Res call({CitationMetadata field0});
}

/// @nodoc
class _$FormatMetadata_CitationCopyWithImpl<$Res>
    implements $FormatMetadata_CitationCopyWith<$Res> {
  _$FormatMetadata_CitationCopyWithImpl(this._self, this._then);

  final FormatMetadata_Citation _self;
  final $Res Function(FormatMetadata_Citation) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Citation(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as CitationMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_FictionBook extends FormatMetadata {
  const FormatMetadata_FictionBook({required this.field0}) : super._();

  @override
  final FictionBookMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_FictionBookCopyWith<FormatMetadata_FictionBook>
      get copyWith =>
          _$FormatMetadata_FictionBookCopyWithImpl<FormatMetadata_FictionBook>(
              this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_FictionBook &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.fictionBook(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_FictionBookCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_FictionBookCopyWith(FormatMetadata_FictionBook value,
          $Res Function(FormatMetadata_FictionBook) _then) =
      _$FormatMetadata_FictionBookCopyWithImpl;
  @useResult
  $Res call({FictionBookMetadata field0});
}

/// @nodoc
class _$FormatMetadata_FictionBookCopyWithImpl<$Res>
    implements $FormatMetadata_FictionBookCopyWith<$Res> {
  _$FormatMetadata_FictionBookCopyWithImpl(this._self, this._then);

  final FormatMetadata_FictionBook _self;
  final $Res Function(FormatMetadata_FictionBook) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_FictionBook(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as FictionBookMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Dbf extends FormatMetadata {
  const FormatMetadata_Dbf({required this.field0}) : super._();

  @override
  final DbfMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_DbfCopyWith<FormatMetadata_Dbf> get copyWith =>
      _$FormatMetadata_DbfCopyWithImpl<FormatMetadata_Dbf>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Dbf &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.dbf(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_DbfCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_DbfCopyWith(
          FormatMetadata_Dbf value, $Res Function(FormatMetadata_Dbf) _then) =
      _$FormatMetadata_DbfCopyWithImpl;
  @useResult
  $Res call({DbfMetadata field0});
}

/// @nodoc
class _$FormatMetadata_DbfCopyWithImpl<$Res>
    implements $FormatMetadata_DbfCopyWith<$Res> {
  _$FormatMetadata_DbfCopyWithImpl(this._self, this._then);

  final FormatMetadata_Dbf _self;
  final $Res Function(FormatMetadata_Dbf) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Dbf(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as DbfMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Jats extends FormatMetadata {
  const FormatMetadata_Jats({required this.field0}) : super._();

  @override
  final JatsMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_JatsCopyWith<FormatMetadata_Jats> get copyWith =>
      _$FormatMetadata_JatsCopyWithImpl<FormatMetadata_Jats>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Jats &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.jats(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_JatsCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_JatsCopyWith(
          FormatMetadata_Jats value, $Res Function(FormatMetadata_Jats) _then) =
      _$FormatMetadata_JatsCopyWithImpl;
  @useResult
  $Res call({JatsMetadata field0});
}

/// @nodoc
class _$FormatMetadata_JatsCopyWithImpl<$Res>
    implements $FormatMetadata_JatsCopyWith<$Res> {
  _$FormatMetadata_JatsCopyWithImpl(this._self, this._then);

  final FormatMetadata_Jats _self;
  final $Res Function(FormatMetadata_Jats) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Jats(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as JatsMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Epub extends FormatMetadata {
  const FormatMetadata_Epub({required this.field0}) : super._();

  @override
  final EpubMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_EpubCopyWith<FormatMetadata_Epub> get copyWith =>
      _$FormatMetadata_EpubCopyWithImpl<FormatMetadata_Epub>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Epub &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.epub(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_EpubCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_EpubCopyWith(
          FormatMetadata_Epub value, $Res Function(FormatMetadata_Epub) _then) =
      _$FormatMetadata_EpubCopyWithImpl;
  @useResult
  $Res call({EpubMetadata field0});
}

/// @nodoc
class _$FormatMetadata_EpubCopyWithImpl<$Res>
    implements $FormatMetadata_EpubCopyWith<$Res> {
  _$FormatMetadata_EpubCopyWithImpl(this._self, this._then);

  final FormatMetadata_Epub _self;
  final $Res Function(FormatMetadata_Epub) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Epub(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as EpubMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Pst extends FormatMetadata {
  const FormatMetadata_Pst({required this.field0}) : super._();

  @override
  final PstMetadata field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_PstCopyWith<FormatMetadata_Pst> get copyWith =>
      _$FormatMetadata_PstCopyWithImpl<FormatMetadata_Pst>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Pst &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.pst(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_PstCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_PstCopyWith(
          FormatMetadata_Pst value, $Res Function(FormatMetadata_Pst) _then) =
      _$FormatMetadata_PstCopyWithImpl;
  @useResult
  $Res call({PstMetadata field0});
}

/// @nodoc
class _$FormatMetadata_PstCopyWithImpl<$Res>
    implements $FormatMetadata_PstCopyWith<$Res> {
  _$FormatMetadata_PstCopyWithImpl(this._self, this._then);

  final FormatMetadata_Pst _self;
  final $Res Function(FormatMetadata_Pst) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Pst(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as PstMetadata,
    ));
  }
}

/// @nodoc

class FormatMetadata_Code extends FormatMetadata {
  const FormatMetadata_Code({required this.field0}) : super._();

  @override
  final String field0;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $FormatMetadata_CodeCopyWith<FormatMetadata_Code> get copyWith =>
      _$FormatMetadata_CodeCopyWithImpl<FormatMetadata_Code>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is FormatMetadata_Code &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'FormatMetadata.code(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $FormatMetadata_CodeCopyWith<$Res>
    implements $FormatMetadataCopyWith<$Res> {
  factory $FormatMetadata_CodeCopyWith(
          FormatMetadata_Code value, $Res Function(FormatMetadata_Code) _then) =
      _$FormatMetadata_CodeCopyWithImpl;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class _$FormatMetadata_CodeCopyWithImpl<$Res>
    implements $FormatMetadata_CodeCopyWith<$Res> {
  _$FormatMetadata_CodeCopyWithImpl(this._self, this._then);

  final FormatMetadata_Code _self;
  final $Res Function(FormatMetadata_Code) _then;

  /// Create a copy of FormatMetadata
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(FormatMetadata_Code(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc
mixin _$NodeContent {
  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is NodeContent);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'NodeContent()';
  }
}

/// @nodoc
class $NodeContentCopyWith<$Res> {
  $NodeContentCopyWith(NodeContent _, $Res Function(NodeContent) __);
}

/// Adds pattern-matching-related methods to [NodeContent].
extension NodeContentPatterns on NodeContent {
  /// A variant of `map` that fallback to returning `orElse`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(NodeContent_Title value)? title,
    TResult Function(NodeContent_Heading value)? heading,
    TResult Function(NodeContent_Paragraph value)? paragraph,
    TResult Function(NodeContent_List value)? list,
    TResult Function(NodeContent_ListItem value)? listItem,
    TResult Function(NodeContent_Table value)? table,
    TResult Function(NodeContent_Image value)? image,
    TResult Function(NodeContent_Code value)? code,
    TResult Function(NodeContent_Quote value)? quote,
    TResult Function(NodeContent_Formula value)? formula,
    TResult Function(NodeContent_Footnote value)? footnote,
    TResult Function(NodeContent_Group value)? group,
    TResult Function(NodeContent_PageBreak value)? pageBreak,
    TResult Function(NodeContent_Slide value)? slide,
    TResult Function(NodeContent_DefinitionList value)? definitionList,
    TResult Function(NodeContent_DefinitionItem value)? definitionItem,
    TResult Function(NodeContent_Citation value)? citation,
    TResult Function(NodeContent_Admonition value)? admonition,
    TResult Function(NodeContent_RawBlock value)? rawBlock,
    TResult Function(NodeContent_MetadataBlock value)? metadataBlock,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case NodeContent_Title() when title != null:
        return title(_that);
      case NodeContent_Heading() when heading != null:
        return heading(_that);
      case NodeContent_Paragraph() when paragraph != null:
        return paragraph(_that);
      case NodeContent_List() when list != null:
        return list(_that);
      case NodeContent_ListItem() when listItem != null:
        return listItem(_that);
      case NodeContent_Table() when table != null:
        return table(_that);
      case NodeContent_Image() when image != null:
        return image(_that);
      case NodeContent_Code() when code != null:
        return code(_that);
      case NodeContent_Quote() when quote != null:
        return quote(_that);
      case NodeContent_Formula() when formula != null:
        return formula(_that);
      case NodeContent_Footnote() when footnote != null:
        return footnote(_that);
      case NodeContent_Group() when group != null:
        return group(_that);
      case NodeContent_PageBreak() when pageBreak != null:
        return pageBreak(_that);
      case NodeContent_Slide() when slide != null:
        return slide(_that);
      case NodeContent_DefinitionList() when definitionList != null:
        return definitionList(_that);
      case NodeContent_DefinitionItem() when definitionItem != null:
        return definitionItem(_that);
      case NodeContent_Citation() when citation != null:
        return citation(_that);
      case NodeContent_Admonition() when admonition != null:
        return admonition(_that);
      case NodeContent_RawBlock() when rawBlock != null:
        return rawBlock(_that);
      case NodeContent_MetadataBlock() when metadataBlock != null:
        return metadataBlock(_that);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// Callbacks receives the raw object, upcasted.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case final Subclass2 value:
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(NodeContent_Title value) title,
    required TResult Function(NodeContent_Heading value) heading,
    required TResult Function(NodeContent_Paragraph value) paragraph,
    required TResult Function(NodeContent_List value) list,
    required TResult Function(NodeContent_ListItem value) listItem,
    required TResult Function(NodeContent_Table value) table,
    required TResult Function(NodeContent_Image value) image,
    required TResult Function(NodeContent_Code value) code,
    required TResult Function(NodeContent_Quote value) quote,
    required TResult Function(NodeContent_Formula value) formula,
    required TResult Function(NodeContent_Footnote value) footnote,
    required TResult Function(NodeContent_Group value) group,
    required TResult Function(NodeContent_PageBreak value) pageBreak,
    required TResult Function(NodeContent_Slide value) slide,
    required TResult Function(NodeContent_DefinitionList value) definitionList,
    required TResult Function(NodeContent_DefinitionItem value) definitionItem,
    required TResult Function(NodeContent_Citation value) citation,
    required TResult Function(NodeContent_Admonition value) admonition,
    required TResult Function(NodeContent_RawBlock value) rawBlock,
    required TResult Function(NodeContent_MetadataBlock value) metadataBlock,
  }) {
    final _that = this;
    switch (_that) {
      case NodeContent_Title():
        return title(_that);
      case NodeContent_Heading():
        return heading(_that);
      case NodeContent_Paragraph():
        return paragraph(_that);
      case NodeContent_List():
        return list(_that);
      case NodeContent_ListItem():
        return listItem(_that);
      case NodeContent_Table():
        return table(_that);
      case NodeContent_Image():
        return image(_that);
      case NodeContent_Code():
        return code(_that);
      case NodeContent_Quote():
        return quote(_that);
      case NodeContent_Formula():
        return formula(_that);
      case NodeContent_Footnote():
        return footnote(_that);
      case NodeContent_Group():
        return group(_that);
      case NodeContent_PageBreak():
        return pageBreak(_that);
      case NodeContent_Slide():
        return slide(_that);
      case NodeContent_DefinitionList():
        return definitionList(_that);
      case NodeContent_DefinitionItem():
        return definitionItem(_that);
      case NodeContent_Citation():
        return citation(_that);
      case NodeContent_Admonition():
        return admonition(_that);
      case NodeContent_RawBlock():
        return rawBlock(_that);
      case NodeContent_MetadataBlock():
        return metadataBlock(_that);
    }
  }

  /// A variant of `map` that fallback to returning `null`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(NodeContent_Title value)? title,
    TResult? Function(NodeContent_Heading value)? heading,
    TResult? Function(NodeContent_Paragraph value)? paragraph,
    TResult? Function(NodeContent_List value)? list,
    TResult? Function(NodeContent_ListItem value)? listItem,
    TResult? Function(NodeContent_Table value)? table,
    TResult? Function(NodeContent_Image value)? image,
    TResult? Function(NodeContent_Code value)? code,
    TResult? Function(NodeContent_Quote value)? quote,
    TResult? Function(NodeContent_Formula value)? formula,
    TResult? Function(NodeContent_Footnote value)? footnote,
    TResult? Function(NodeContent_Group value)? group,
    TResult? Function(NodeContent_PageBreak value)? pageBreak,
    TResult? Function(NodeContent_Slide value)? slide,
    TResult? Function(NodeContent_DefinitionList value)? definitionList,
    TResult? Function(NodeContent_DefinitionItem value)? definitionItem,
    TResult? Function(NodeContent_Citation value)? citation,
    TResult? Function(NodeContent_Admonition value)? admonition,
    TResult? Function(NodeContent_RawBlock value)? rawBlock,
    TResult? Function(NodeContent_MetadataBlock value)? metadataBlock,
  }) {
    final _that = this;
    switch (_that) {
      case NodeContent_Title() when title != null:
        return title(_that);
      case NodeContent_Heading() when heading != null:
        return heading(_that);
      case NodeContent_Paragraph() when paragraph != null:
        return paragraph(_that);
      case NodeContent_List() when list != null:
        return list(_that);
      case NodeContent_ListItem() when listItem != null:
        return listItem(_that);
      case NodeContent_Table() when table != null:
        return table(_that);
      case NodeContent_Image() when image != null:
        return image(_that);
      case NodeContent_Code() when code != null:
        return code(_that);
      case NodeContent_Quote() when quote != null:
        return quote(_that);
      case NodeContent_Formula() when formula != null:
        return formula(_that);
      case NodeContent_Footnote() when footnote != null:
        return footnote(_that);
      case NodeContent_Group() when group != null:
        return group(_that);
      case NodeContent_PageBreak() when pageBreak != null:
        return pageBreak(_that);
      case NodeContent_Slide() when slide != null:
        return slide(_that);
      case NodeContent_DefinitionList() when definitionList != null:
        return definitionList(_that);
      case NodeContent_DefinitionItem() when definitionItem != null:
        return definitionItem(_that);
      case NodeContent_Citation() when citation != null:
        return citation(_that);
      case NodeContent_Admonition() when admonition != null:
        return admonition(_that);
      case NodeContent_RawBlock() when rawBlock != null:
        return rawBlock(_that);
      case NodeContent_MetadataBlock() when metadataBlock != null:
        return metadataBlock(_that);
      case _:
        return null;
    }
  }

  /// A variant of `when` that fallback to an `orElse` callback.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String text)? title,
    TResult Function(PlatformInt64 level, String text)? heading,
    TResult Function(String text)? paragraph,
    TResult Function(bool ordered)? list,
    TResult Function(String text)? listItem,
    TResult Function(TableGrid grid)? table,
    TResult Function(String description, PlatformInt64 imageIndex, String src)?
        image,
    TResult Function(String text, String language)? code,
    TResult Function()? quote,
    TResult Function(String text)? formula,
    TResult Function(String text)? footnote,
    TResult Function(
            String label, PlatformInt64 headingLevel, String headingText)?
        group,
    TResult Function()? pageBreak,
    TResult Function(PlatformInt64 number, String title)? slide,
    TResult Function()? definitionList,
    TResult Function(String term, String definition)? definitionItem,
    TResult Function(String key, String text)? citation,
    TResult Function(String kind, String title)? admonition,
    TResult Function(String format, String content)? rawBlock,
    TResult Function(List<String> entries)? metadataBlock,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case NodeContent_Title() when title != null:
        return title(_that.text);
      case NodeContent_Heading() when heading != null:
        return heading(_that.level, _that.text);
      case NodeContent_Paragraph() when paragraph != null:
        return paragraph(_that.text);
      case NodeContent_List() when list != null:
        return list(_that.ordered);
      case NodeContent_ListItem() when listItem != null:
        return listItem(_that.text);
      case NodeContent_Table() when table != null:
        return table(_that.grid);
      case NodeContent_Image() when image != null:
        return image(_that.description, _that.imageIndex, _that.src);
      case NodeContent_Code() when code != null:
        return code(_that.text, _that.language);
      case NodeContent_Quote() when quote != null:
        return quote();
      case NodeContent_Formula() when formula != null:
        return formula(_that.text);
      case NodeContent_Footnote() when footnote != null:
        return footnote(_that.text);
      case NodeContent_Group() when group != null:
        return group(_that.label, _that.headingLevel, _that.headingText);
      case NodeContent_PageBreak() when pageBreak != null:
        return pageBreak();
      case NodeContent_Slide() when slide != null:
        return slide(_that.number, _that.title);
      case NodeContent_DefinitionList() when definitionList != null:
        return definitionList();
      case NodeContent_DefinitionItem() when definitionItem != null:
        return definitionItem(_that.term, _that.definition);
      case NodeContent_Citation() when citation != null:
        return citation(_that.key, _that.text);
      case NodeContent_Admonition() when admonition != null:
        return admonition(_that.kind, _that.title);
      case NodeContent_RawBlock() when rawBlock != null:
        return rawBlock(_that.format, _that.content);
      case NodeContent_MetadataBlock() when metadataBlock != null:
        return metadataBlock(_that.entries);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// As opposed to `map`, this offers destructuring.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case Subclass2(:final field2):
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String text) title,
    required TResult Function(PlatformInt64 level, String text) heading,
    required TResult Function(String text) paragraph,
    required TResult Function(bool ordered) list,
    required TResult Function(String text) listItem,
    required TResult Function(TableGrid grid) table,
    required TResult Function(
            String description, PlatformInt64 imageIndex, String src)
        image,
    required TResult Function(String text, String language) code,
    required TResult Function() quote,
    required TResult Function(String text) formula,
    required TResult Function(String text) footnote,
    required TResult Function(
            String label, PlatformInt64 headingLevel, String headingText)
        group,
    required TResult Function() pageBreak,
    required TResult Function(PlatformInt64 number, String title) slide,
    required TResult Function() definitionList,
    required TResult Function(String term, String definition) definitionItem,
    required TResult Function(String key, String text) citation,
    required TResult Function(String kind, String title) admonition,
    required TResult Function(String format, String content) rawBlock,
    required TResult Function(List<String> entries) metadataBlock,
  }) {
    final _that = this;
    switch (_that) {
      case NodeContent_Title():
        return title(_that.text);
      case NodeContent_Heading():
        return heading(_that.level, _that.text);
      case NodeContent_Paragraph():
        return paragraph(_that.text);
      case NodeContent_List():
        return list(_that.ordered);
      case NodeContent_ListItem():
        return listItem(_that.text);
      case NodeContent_Table():
        return table(_that.grid);
      case NodeContent_Image():
        return image(_that.description, _that.imageIndex, _that.src);
      case NodeContent_Code():
        return code(_that.text, _that.language);
      case NodeContent_Quote():
        return quote();
      case NodeContent_Formula():
        return formula(_that.text);
      case NodeContent_Footnote():
        return footnote(_that.text);
      case NodeContent_Group():
        return group(_that.label, _that.headingLevel, _that.headingText);
      case NodeContent_PageBreak():
        return pageBreak();
      case NodeContent_Slide():
        return slide(_that.number, _that.title);
      case NodeContent_DefinitionList():
        return definitionList();
      case NodeContent_DefinitionItem():
        return definitionItem(_that.term, _that.definition);
      case NodeContent_Citation():
        return citation(_that.key, _that.text);
      case NodeContent_Admonition():
        return admonition(_that.kind, _that.title);
      case NodeContent_RawBlock():
        return rawBlock(_that.format, _that.content);
      case NodeContent_MetadataBlock():
        return metadataBlock(_that.entries);
    }
  }

  /// A variant of `when` that fallback to returning `null`
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String text)? title,
    TResult? Function(PlatformInt64 level, String text)? heading,
    TResult? Function(String text)? paragraph,
    TResult? Function(bool ordered)? list,
    TResult? Function(String text)? listItem,
    TResult? Function(TableGrid grid)? table,
    TResult? Function(String description, PlatformInt64 imageIndex, String src)?
        image,
    TResult? Function(String text, String language)? code,
    TResult? Function()? quote,
    TResult? Function(String text)? formula,
    TResult? Function(String text)? footnote,
    TResult? Function(
            String label, PlatformInt64 headingLevel, String headingText)?
        group,
    TResult? Function()? pageBreak,
    TResult? Function(PlatformInt64 number, String title)? slide,
    TResult? Function()? definitionList,
    TResult? Function(String term, String definition)? definitionItem,
    TResult? Function(String key, String text)? citation,
    TResult? Function(String kind, String title)? admonition,
    TResult? Function(String format, String content)? rawBlock,
    TResult? Function(List<String> entries)? metadataBlock,
  }) {
    final _that = this;
    switch (_that) {
      case NodeContent_Title() when title != null:
        return title(_that.text);
      case NodeContent_Heading() when heading != null:
        return heading(_that.level, _that.text);
      case NodeContent_Paragraph() when paragraph != null:
        return paragraph(_that.text);
      case NodeContent_List() when list != null:
        return list(_that.ordered);
      case NodeContent_ListItem() when listItem != null:
        return listItem(_that.text);
      case NodeContent_Table() when table != null:
        return table(_that.grid);
      case NodeContent_Image() when image != null:
        return image(_that.description, _that.imageIndex, _that.src);
      case NodeContent_Code() when code != null:
        return code(_that.text, _that.language);
      case NodeContent_Quote() when quote != null:
        return quote();
      case NodeContent_Formula() when formula != null:
        return formula(_that.text);
      case NodeContent_Footnote() when footnote != null:
        return footnote(_that.text);
      case NodeContent_Group() when group != null:
        return group(_that.label, _that.headingLevel, _that.headingText);
      case NodeContent_PageBreak() when pageBreak != null:
        return pageBreak();
      case NodeContent_Slide() when slide != null:
        return slide(_that.number, _that.title);
      case NodeContent_DefinitionList() when definitionList != null:
        return definitionList();
      case NodeContent_DefinitionItem() when definitionItem != null:
        return definitionItem(_that.term, _that.definition);
      case NodeContent_Citation() when citation != null:
        return citation(_that.key, _that.text);
      case NodeContent_Admonition() when admonition != null:
        return admonition(_that.kind, _that.title);
      case NodeContent_RawBlock() when rawBlock != null:
        return rawBlock(_that.format, _that.content);
      case NodeContent_MetadataBlock() when metadataBlock != null:
        return metadataBlock(_that.entries);
      case _:
        return null;
    }
  }
}

/// @nodoc

class NodeContent_Title extends NodeContent {
  const NodeContent_Title({required this.text}) : super._();

  final String text;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_TitleCopyWith<NodeContent_Title> get copyWith =>
      _$NodeContent_TitleCopyWithImpl<NodeContent_Title>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_Title &&
            (identical(other.text, text) || other.text == text));
  }

  @override
  int get hashCode => Object.hash(runtimeType, text);

  @override
  String toString() {
    return 'NodeContent.title(text: $text)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_TitleCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_TitleCopyWith(
          NodeContent_Title value, $Res Function(NodeContent_Title) _then) =
      _$NodeContent_TitleCopyWithImpl;
  @useResult
  $Res call({String text});
}

/// @nodoc
class _$NodeContent_TitleCopyWithImpl<$Res>
    implements $NodeContent_TitleCopyWith<$Res> {
  _$NodeContent_TitleCopyWithImpl(this._self, this._then);

  final NodeContent_Title _self;
  final $Res Function(NodeContent_Title) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? text = null,
  }) {
    return _then(NodeContent_Title(
      text: null == text
          ? _self.text
          : text // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_Heading extends NodeContent {
  const NodeContent_Heading({required this.level, required this.text})
      : super._();

  final PlatformInt64 level;
  final String text;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_HeadingCopyWith<NodeContent_Heading> get copyWith =>
      _$NodeContent_HeadingCopyWithImpl<NodeContent_Heading>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_Heading &&
            (identical(other.level, level) || other.level == level) &&
            (identical(other.text, text) || other.text == text));
  }

  @override
  int get hashCode => Object.hash(runtimeType, level, text);

  @override
  String toString() {
    return 'NodeContent.heading(level: $level, text: $text)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_HeadingCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_HeadingCopyWith(
          NodeContent_Heading value, $Res Function(NodeContent_Heading) _then) =
      _$NodeContent_HeadingCopyWithImpl;
  @useResult
  $Res call({PlatformInt64 level, String text});
}

/// @nodoc
class _$NodeContent_HeadingCopyWithImpl<$Res>
    implements $NodeContent_HeadingCopyWith<$Res> {
  _$NodeContent_HeadingCopyWithImpl(this._self, this._then);

  final NodeContent_Heading _self;
  final $Res Function(NodeContent_Heading) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? level = null,
    Object? text = null,
  }) {
    return _then(NodeContent_Heading(
      level: null == level
          ? _self.level
          : level // ignore: cast_nullable_to_non_nullable
              as PlatformInt64,
      text: null == text
          ? _self.text
          : text // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_Paragraph extends NodeContent {
  const NodeContent_Paragraph({required this.text}) : super._();

  final String text;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_ParagraphCopyWith<NodeContent_Paragraph> get copyWith =>
      _$NodeContent_ParagraphCopyWithImpl<NodeContent_Paragraph>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_Paragraph &&
            (identical(other.text, text) || other.text == text));
  }

  @override
  int get hashCode => Object.hash(runtimeType, text);

  @override
  String toString() {
    return 'NodeContent.paragraph(text: $text)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_ParagraphCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_ParagraphCopyWith(NodeContent_Paragraph value,
          $Res Function(NodeContent_Paragraph) _then) =
      _$NodeContent_ParagraphCopyWithImpl;
  @useResult
  $Res call({String text});
}

/// @nodoc
class _$NodeContent_ParagraphCopyWithImpl<$Res>
    implements $NodeContent_ParagraphCopyWith<$Res> {
  _$NodeContent_ParagraphCopyWithImpl(this._self, this._then);

  final NodeContent_Paragraph _self;
  final $Res Function(NodeContent_Paragraph) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? text = null,
  }) {
    return _then(NodeContent_Paragraph(
      text: null == text
          ? _self.text
          : text // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_List extends NodeContent {
  const NodeContent_List({required this.ordered}) : super._();

  final bool ordered;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_ListCopyWith<NodeContent_List> get copyWith =>
      _$NodeContent_ListCopyWithImpl<NodeContent_List>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_List &&
            (identical(other.ordered, ordered) || other.ordered == ordered));
  }

  @override
  int get hashCode => Object.hash(runtimeType, ordered);

  @override
  String toString() {
    return 'NodeContent.list(ordered: $ordered)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_ListCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_ListCopyWith(
          NodeContent_List value, $Res Function(NodeContent_List) _then) =
      _$NodeContent_ListCopyWithImpl;
  @useResult
  $Res call({bool ordered});
}

/// @nodoc
class _$NodeContent_ListCopyWithImpl<$Res>
    implements $NodeContent_ListCopyWith<$Res> {
  _$NodeContent_ListCopyWithImpl(this._self, this._then);

  final NodeContent_List _self;
  final $Res Function(NodeContent_List) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? ordered = null,
  }) {
    return _then(NodeContent_List(
      ordered: null == ordered
          ? _self.ordered
          : ordered // ignore: cast_nullable_to_non_nullable
              as bool,
    ));
  }
}

/// @nodoc

class NodeContent_ListItem extends NodeContent {
  const NodeContent_ListItem({required this.text}) : super._();

  final String text;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_ListItemCopyWith<NodeContent_ListItem> get copyWith =>
      _$NodeContent_ListItemCopyWithImpl<NodeContent_ListItem>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_ListItem &&
            (identical(other.text, text) || other.text == text));
  }

  @override
  int get hashCode => Object.hash(runtimeType, text);

  @override
  String toString() {
    return 'NodeContent.listItem(text: $text)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_ListItemCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_ListItemCopyWith(NodeContent_ListItem value,
          $Res Function(NodeContent_ListItem) _then) =
      _$NodeContent_ListItemCopyWithImpl;
  @useResult
  $Res call({String text});
}

/// @nodoc
class _$NodeContent_ListItemCopyWithImpl<$Res>
    implements $NodeContent_ListItemCopyWith<$Res> {
  _$NodeContent_ListItemCopyWithImpl(this._self, this._then);

  final NodeContent_ListItem _self;
  final $Res Function(NodeContent_ListItem) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? text = null,
  }) {
    return _then(NodeContent_ListItem(
      text: null == text
          ? _self.text
          : text // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_Table extends NodeContent {
  const NodeContent_Table({required this.grid}) : super._();

  final TableGrid grid;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_TableCopyWith<NodeContent_Table> get copyWith =>
      _$NodeContent_TableCopyWithImpl<NodeContent_Table>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_Table &&
            (identical(other.grid, grid) || other.grid == grid));
  }

  @override
  int get hashCode => Object.hash(runtimeType, grid);

  @override
  String toString() {
    return 'NodeContent.table(grid: $grid)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_TableCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_TableCopyWith(
          NodeContent_Table value, $Res Function(NodeContent_Table) _then) =
      _$NodeContent_TableCopyWithImpl;
  @useResult
  $Res call({TableGrid grid});
}

/// @nodoc
class _$NodeContent_TableCopyWithImpl<$Res>
    implements $NodeContent_TableCopyWith<$Res> {
  _$NodeContent_TableCopyWithImpl(this._self, this._then);

  final NodeContent_Table _self;
  final $Res Function(NodeContent_Table) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? grid = null,
  }) {
    return _then(NodeContent_Table(
      grid: null == grid
          ? _self.grid
          : grid // ignore: cast_nullable_to_non_nullable
              as TableGrid,
    ));
  }
}

/// @nodoc

class NodeContent_Image extends NodeContent {
  const NodeContent_Image(
      {required this.description, required this.imageIndex, required this.src})
      : super._();

  final String description;
  final PlatformInt64 imageIndex;
  final String src;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_ImageCopyWith<NodeContent_Image> get copyWith =>
      _$NodeContent_ImageCopyWithImpl<NodeContent_Image>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_Image &&
            (identical(other.description, description) ||
                other.description == description) &&
            (identical(other.imageIndex, imageIndex) ||
                other.imageIndex == imageIndex) &&
            (identical(other.src, src) || other.src == src));
  }

  @override
  int get hashCode => Object.hash(runtimeType, description, imageIndex, src);

  @override
  String toString() {
    return 'NodeContent.image(description: $description, imageIndex: $imageIndex, src: $src)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_ImageCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_ImageCopyWith(
          NodeContent_Image value, $Res Function(NodeContent_Image) _then) =
      _$NodeContent_ImageCopyWithImpl;
  @useResult
  $Res call({String description, PlatformInt64 imageIndex, String src});
}

/// @nodoc
class _$NodeContent_ImageCopyWithImpl<$Res>
    implements $NodeContent_ImageCopyWith<$Res> {
  _$NodeContent_ImageCopyWithImpl(this._self, this._then);

  final NodeContent_Image _self;
  final $Res Function(NodeContent_Image) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? description = null,
    Object? imageIndex = null,
    Object? src = null,
  }) {
    return _then(NodeContent_Image(
      description: null == description
          ? _self.description
          : description // ignore: cast_nullable_to_non_nullable
              as String,
      imageIndex: null == imageIndex
          ? _self.imageIndex
          : imageIndex // ignore: cast_nullable_to_non_nullable
              as PlatformInt64,
      src: null == src
          ? _self.src
          : src // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_Code extends NodeContent {
  const NodeContent_Code({required this.text, required this.language})
      : super._();

  final String text;
  final String language;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_CodeCopyWith<NodeContent_Code> get copyWith =>
      _$NodeContent_CodeCopyWithImpl<NodeContent_Code>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_Code &&
            (identical(other.text, text) || other.text == text) &&
            (identical(other.language, language) ||
                other.language == language));
  }

  @override
  int get hashCode => Object.hash(runtimeType, text, language);

  @override
  String toString() {
    return 'NodeContent.code(text: $text, language: $language)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_CodeCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_CodeCopyWith(
          NodeContent_Code value, $Res Function(NodeContent_Code) _then) =
      _$NodeContent_CodeCopyWithImpl;
  @useResult
  $Res call({String text, String language});
}

/// @nodoc
class _$NodeContent_CodeCopyWithImpl<$Res>
    implements $NodeContent_CodeCopyWith<$Res> {
  _$NodeContent_CodeCopyWithImpl(this._self, this._then);

  final NodeContent_Code _self;
  final $Res Function(NodeContent_Code) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? text = null,
    Object? language = null,
  }) {
    return _then(NodeContent_Code(
      text: null == text
          ? _self.text
          : text // ignore: cast_nullable_to_non_nullable
              as String,
      language: null == language
          ? _self.language
          : language // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_Quote extends NodeContent {
  const NodeContent_Quote() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is NodeContent_Quote);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'NodeContent.quote()';
  }
}

/// @nodoc

class NodeContent_Formula extends NodeContent {
  const NodeContent_Formula({required this.text}) : super._();

  final String text;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_FormulaCopyWith<NodeContent_Formula> get copyWith =>
      _$NodeContent_FormulaCopyWithImpl<NodeContent_Formula>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_Formula &&
            (identical(other.text, text) || other.text == text));
  }

  @override
  int get hashCode => Object.hash(runtimeType, text);

  @override
  String toString() {
    return 'NodeContent.formula(text: $text)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_FormulaCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_FormulaCopyWith(
          NodeContent_Formula value, $Res Function(NodeContent_Formula) _then) =
      _$NodeContent_FormulaCopyWithImpl;
  @useResult
  $Res call({String text});
}

/// @nodoc
class _$NodeContent_FormulaCopyWithImpl<$Res>
    implements $NodeContent_FormulaCopyWith<$Res> {
  _$NodeContent_FormulaCopyWithImpl(this._self, this._then);

  final NodeContent_Formula _self;
  final $Res Function(NodeContent_Formula) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? text = null,
  }) {
    return _then(NodeContent_Formula(
      text: null == text
          ? _self.text
          : text // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_Footnote extends NodeContent {
  const NodeContent_Footnote({required this.text}) : super._();

  final String text;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_FootnoteCopyWith<NodeContent_Footnote> get copyWith =>
      _$NodeContent_FootnoteCopyWithImpl<NodeContent_Footnote>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_Footnote &&
            (identical(other.text, text) || other.text == text));
  }

  @override
  int get hashCode => Object.hash(runtimeType, text);

  @override
  String toString() {
    return 'NodeContent.footnote(text: $text)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_FootnoteCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_FootnoteCopyWith(NodeContent_Footnote value,
          $Res Function(NodeContent_Footnote) _then) =
      _$NodeContent_FootnoteCopyWithImpl;
  @useResult
  $Res call({String text});
}

/// @nodoc
class _$NodeContent_FootnoteCopyWithImpl<$Res>
    implements $NodeContent_FootnoteCopyWith<$Res> {
  _$NodeContent_FootnoteCopyWithImpl(this._self, this._then);

  final NodeContent_Footnote _self;
  final $Res Function(NodeContent_Footnote) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? text = null,
  }) {
    return _then(NodeContent_Footnote(
      text: null == text
          ? _self.text
          : text // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_Group extends NodeContent {
  const NodeContent_Group(
      {required this.label,
      required this.headingLevel,
      required this.headingText})
      : super._();

  final String label;
  final PlatformInt64 headingLevel;
  final String headingText;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_GroupCopyWith<NodeContent_Group> get copyWith =>
      _$NodeContent_GroupCopyWithImpl<NodeContent_Group>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_Group &&
            (identical(other.label, label) || other.label == label) &&
            (identical(other.headingLevel, headingLevel) ||
                other.headingLevel == headingLevel) &&
            (identical(other.headingText, headingText) ||
                other.headingText == headingText));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, label, headingLevel, headingText);

  @override
  String toString() {
    return 'NodeContent.group(label: $label, headingLevel: $headingLevel, headingText: $headingText)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_GroupCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_GroupCopyWith(
          NodeContent_Group value, $Res Function(NodeContent_Group) _then) =
      _$NodeContent_GroupCopyWithImpl;
  @useResult
  $Res call({String label, PlatformInt64 headingLevel, String headingText});
}

/// @nodoc
class _$NodeContent_GroupCopyWithImpl<$Res>
    implements $NodeContent_GroupCopyWith<$Res> {
  _$NodeContent_GroupCopyWithImpl(this._self, this._then);

  final NodeContent_Group _self;
  final $Res Function(NodeContent_Group) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? label = null,
    Object? headingLevel = null,
    Object? headingText = null,
  }) {
    return _then(NodeContent_Group(
      label: null == label
          ? _self.label
          : label // ignore: cast_nullable_to_non_nullable
              as String,
      headingLevel: null == headingLevel
          ? _self.headingLevel
          : headingLevel // ignore: cast_nullable_to_non_nullable
              as PlatformInt64,
      headingText: null == headingText
          ? _self.headingText
          : headingText // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_PageBreak extends NodeContent {
  const NodeContent_PageBreak() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is NodeContent_PageBreak);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'NodeContent.pageBreak()';
  }
}

/// @nodoc

class NodeContent_Slide extends NodeContent {
  const NodeContent_Slide({required this.number, required this.title})
      : super._();

  final PlatformInt64 number;
  final String title;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_SlideCopyWith<NodeContent_Slide> get copyWith =>
      _$NodeContent_SlideCopyWithImpl<NodeContent_Slide>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_Slide &&
            (identical(other.number, number) || other.number == number) &&
            (identical(other.title, title) || other.title == title));
  }

  @override
  int get hashCode => Object.hash(runtimeType, number, title);

  @override
  String toString() {
    return 'NodeContent.slide(number: $number, title: $title)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_SlideCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_SlideCopyWith(
          NodeContent_Slide value, $Res Function(NodeContent_Slide) _then) =
      _$NodeContent_SlideCopyWithImpl;
  @useResult
  $Res call({PlatformInt64 number, String title});
}

/// @nodoc
class _$NodeContent_SlideCopyWithImpl<$Res>
    implements $NodeContent_SlideCopyWith<$Res> {
  _$NodeContent_SlideCopyWithImpl(this._self, this._then);

  final NodeContent_Slide _self;
  final $Res Function(NodeContent_Slide) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? number = null,
    Object? title = null,
  }) {
    return _then(NodeContent_Slide(
      number: null == number
          ? _self.number
          : number // ignore: cast_nullable_to_non_nullable
              as PlatformInt64,
      title: null == title
          ? _self.title
          : title // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_DefinitionList extends NodeContent {
  const NodeContent_DefinitionList() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_DefinitionList);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'NodeContent.definitionList()';
  }
}

/// @nodoc

class NodeContent_DefinitionItem extends NodeContent {
  const NodeContent_DefinitionItem(
      {required this.term, required this.definition})
      : super._();

  final String term;
  final String definition;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_DefinitionItemCopyWith<NodeContent_DefinitionItem>
      get copyWith =>
          _$NodeContent_DefinitionItemCopyWithImpl<NodeContent_DefinitionItem>(
              this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_DefinitionItem &&
            (identical(other.term, term) || other.term == term) &&
            (identical(other.definition, definition) ||
                other.definition == definition));
  }

  @override
  int get hashCode => Object.hash(runtimeType, term, definition);

  @override
  String toString() {
    return 'NodeContent.definitionItem(term: $term, definition: $definition)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_DefinitionItemCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_DefinitionItemCopyWith(NodeContent_DefinitionItem value,
          $Res Function(NodeContent_DefinitionItem) _then) =
      _$NodeContent_DefinitionItemCopyWithImpl;
  @useResult
  $Res call({String term, String definition});
}

/// @nodoc
class _$NodeContent_DefinitionItemCopyWithImpl<$Res>
    implements $NodeContent_DefinitionItemCopyWith<$Res> {
  _$NodeContent_DefinitionItemCopyWithImpl(this._self, this._then);

  final NodeContent_DefinitionItem _self;
  final $Res Function(NodeContent_DefinitionItem) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? term = null,
    Object? definition = null,
  }) {
    return _then(NodeContent_DefinitionItem(
      term: null == term
          ? _self.term
          : term // ignore: cast_nullable_to_non_nullable
              as String,
      definition: null == definition
          ? _self.definition
          : definition // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_Citation extends NodeContent {
  const NodeContent_Citation({required this.key, required this.text})
      : super._();

  final String key;
  final String text;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_CitationCopyWith<NodeContent_Citation> get copyWith =>
      _$NodeContent_CitationCopyWithImpl<NodeContent_Citation>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_Citation &&
            (identical(other.key, key) || other.key == key) &&
            (identical(other.text, text) || other.text == text));
  }

  @override
  int get hashCode => Object.hash(runtimeType, key, text);

  @override
  String toString() {
    return 'NodeContent.citation(key: $key, text: $text)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_CitationCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_CitationCopyWith(NodeContent_Citation value,
          $Res Function(NodeContent_Citation) _then) =
      _$NodeContent_CitationCopyWithImpl;
  @useResult
  $Res call({String key, String text});
}

/// @nodoc
class _$NodeContent_CitationCopyWithImpl<$Res>
    implements $NodeContent_CitationCopyWith<$Res> {
  _$NodeContent_CitationCopyWithImpl(this._self, this._then);

  final NodeContent_Citation _self;
  final $Res Function(NodeContent_Citation) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? key = null,
    Object? text = null,
  }) {
    return _then(NodeContent_Citation(
      key: null == key
          ? _self.key
          : key // ignore: cast_nullable_to_non_nullable
              as String,
      text: null == text
          ? _self.text
          : text // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_Admonition extends NodeContent {
  const NodeContent_Admonition({required this.kind, required this.title})
      : super._();

  final String kind;
  final String title;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_AdmonitionCopyWith<NodeContent_Admonition> get copyWith =>
      _$NodeContent_AdmonitionCopyWithImpl<NodeContent_Admonition>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_Admonition &&
            (identical(other.kind, kind) || other.kind == kind) &&
            (identical(other.title, title) || other.title == title));
  }

  @override
  int get hashCode => Object.hash(runtimeType, kind, title);

  @override
  String toString() {
    return 'NodeContent.admonition(kind: $kind, title: $title)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_AdmonitionCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_AdmonitionCopyWith(NodeContent_Admonition value,
          $Res Function(NodeContent_Admonition) _then) =
      _$NodeContent_AdmonitionCopyWithImpl;
  @useResult
  $Res call({String kind, String title});
}

/// @nodoc
class _$NodeContent_AdmonitionCopyWithImpl<$Res>
    implements $NodeContent_AdmonitionCopyWith<$Res> {
  _$NodeContent_AdmonitionCopyWithImpl(this._self, this._then);

  final NodeContent_Admonition _self;
  final $Res Function(NodeContent_Admonition) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? kind = null,
    Object? title = null,
  }) {
    return _then(NodeContent_Admonition(
      kind: null == kind
          ? _self.kind
          : kind // ignore: cast_nullable_to_non_nullable
              as String,
      title: null == title
          ? _self.title
          : title // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_RawBlock extends NodeContent {
  const NodeContent_RawBlock({required this.format, required this.content})
      : super._();

  final String format;
  final String content;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_RawBlockCopyWith<NodeContent_RawBlock> get copyWith =>
      _$NodeContent_RawBlockCopyWithImpl<NodeContent_RawBlock>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_RawBlock &&
            (identical(other.format, format) || other.format == format) &&
            (identical(other.content, content) || other.content == content));
  }

  @override
  int get hashCode => Object.hash(runtimeType, format, content);

  @override
  String toString() {
    return 'NodeContent.rawBlock(format: $format, content: $content)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_RawBlockCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_RawBlockCopyWith(NodeContent_RawBlock value,
          $Res Function(NodeContent_RawBlock) _then) =
      _$NodeContent_RawBlockCopyWithImpl;
  @useResult
  $Res call({String format, String content});
}

/// @nodoc
class _$NodeContent_RawBlockCopyWithImpl<$Res>
    implements $NodeContent_RawBlockCopyWith<$Res> {
  _$NodeContent_RawBlockCopyWithImpl(this._self, this._then);

  final NodeContent_RawBlock _self;
  final $Res Function(NodeContent_RawBlock) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? format = null,
    Object? content = null,
  }) {
    return _then(NodeContent_RawBlock(
      format: null == format
          ? _self.format
          : format // ignore: cast_nullable_to_non_nullable
              as String,
      content: null == content
          ? _self.content
          : content // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class NodeContent_MetadataBlock extends NodeContent {
  const NodeContent_MetadataBlock({required final List<String> entries})
      : _entries = entries,
        super._();

  final List<String> _entries;
  List<String> get entries {
    if (_entries is EqualUnmodifiableListView) return _entries;
    // ignore: implicit_dynamic_type
    return EqualUnmodifiableListView(_entries);
  }

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $NodeContent_MetadataBlockCopyWith<NodeContent_MetadataBlock> get copyWith =>
      _$NodeContent_MetadataBlockCopyWithImpl<NodeContent_MetadataBlock>(
          this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is NodeContent_MetadataBlock &&
            const DeepCollectionEquality().equals(other._entries, _entries));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(_entries));

  @override
  String toString() {
    return 'NodeContent.metadataBlock(entries: $entries)';
  }
}

/// @nodoc
abstract mixin class $NodeContent_MetadataBlockCopyWith<$Res>
    implements $NodeContentCopyWith<$Res> {
  factory $NodeContent_MetadataBlockCopyWith(NodeContent_MetadataBlock value,
          $Res Function(NodeContent_MetadataBlock) _then) =
      _$NodeContent_MetadataBlockCopyWithImpl;
  @useResult
  $Res call({List<String> entries});
}

/// @nodoc
class _$NodeContent_MetadataBlockCopyWithImpl<$Res>
    implements $NodeContent_MetadataBlockCopyWith<$Res> {
  _$NodeContent_MetadataBlockCopyWithImpl(this._self, this._then);

  final NodeContent_MetadataBlock _self;
  final $Res Function(NodeContent_MetadataBlock) _then;

  /// Create a copy of NodeContent
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? entries = null,
  }) {
    return _then(NodeContent_MetadataBlock(
      entries: null == entries
          ? _self._entries
          : entries // ignore: cast_nullable_to_non_nullable
              as List<String>,
    ));
  }
}

/// @nodoc
mixin _$OcrBoundingGeometry {
  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is OcrBoundingGeometry);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'OcrBoundingGeometry()';
  }
}

/// @nodoc
class $OcrBoundingGeometryCopyWith<$Res> {
  $OcrBoundingGeometryCopyWith(
      OcrBoundingGeometry _, $Res Function(OcrBoundingGeometry) __);
}

/// Adds pattern-matching-related methods to [OcrBoundingGeometry].
extension OcrBoundingGeometryPatterns on OcrBoundingGeometry {
  /// A variant of `map` that fallback to returning `orElse`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OcrBoundingGeometry_Rectangle value)? rectangle,
    TResult Function(OcrBoundingGeometry_Quadrilateral value)? quadrilateral,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case OcrBoundingGeometry_Rectangle() when rectangle != null:
        return rectangle(_that);
      case OcrBoundingGeometry_Quadrilateral() when quadrilateral != null:
        return quadrilateral(_that);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// Callbacks receives the raw object, upcasted.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case final Subclass2 value:
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OcrBoundingGeometry_Rectangle value) rectangle,
    required TResult Function(OcrBoundingGeometry_Quadrilateral value)
        quadrilateral,
  }) {
    final _that = this;
    switch (_that) {
      case OcrBoundingGeometry_Rectangle():
        return rectangle(_that);
      case OcrBoundingGeometry_Quadrilateral():
        return quadrilateral(_that);
    }
  }

  /// A variant of `map` that fallback to returning `null`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OcrBoundingGeometry_Rectangle value)? rectangle,
    TResult? Function(OcrBoundingGeometry_Quadrilateral value)? quadrilateral,
  }) {
    final _that = this;
    switch (_that) {
      case OcrBoundingGeometry_Rectangle() when rectangle != null:
        return rectangle(_that);
      case OcrBoundingGeometry_Quadrilateral() when quadrilateral != null:
        return quadrilateral(_that);
      case _:
        return null;
    }
  }

  /// A variant of `when` that fallback to an `orElse` callback.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(PlatformInt64 left, PlatformInt64 top, PlatformInt64 width,
            PlatformInt64 height)?
        rectangle,
    TResult Function(String points)? quadrilateral,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case OcrBoundingGeometry_Rectangle() when rectangle != null:
        return rectangle(_that.left, _that.top, _that.width, _that.height);
      case OcrBoundingGeometry_Quadrilateral() when quadrilateral != null:
        return quadrilateral(_that.points);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// As opposed to `map`, this offers destructuring.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case Subclass2(:final field2):
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(PlatformInt64 left, PlatformInt64 top,
            PlatformInt64 width, PlatformInt64 height)
        rectangle,
    required TResult Function(String points) quadrilateral,
  }) {
    final _that = this;
    switch (_that) {
      case OcrBoundingGeometry_Rectangle():
        return rectangle(_that.left, _that.top, _that.width, _that.height);
      case OcrBoundingGeometry_Quadrilateral():
        return quadrilateral(_that.points);
    }
  }

  /// A variant of `when` that fallback to returning `null`
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(PlatformInt64 left, PlatformInt64 top,
            PlatformInt64 width, PlatformInt64 height)?
        rectangle,
    TResult? Function(String points)? quadrilateral,
  }) {
    final _that = this;
    switch (_that) {
      case OcrBoundingGeometry_Rectangle() when rectangle != null:
        return rectangle(_that.left, _that.top, _that.width, _that.height);
      case OcrBoundingGeometry_Quadrilateral() when quadrilateral != null:
        return quadrilateral(_that.points);
      case _:
        return null;
    }
  }
}

/// @nodoc

class OcrBoundingGeometry_Rectangle extends OcrBoundingGeometry {
  const OcrBoundingGeometry_Rectangle(
      {required this.left,
      required this.top,
      required this.width,
      required this.height})
      : super._();

  final PlatformInt64 left;
  final PlatformInt64 top;
  final PlatformInt64 width;
  final PlatformInt64 height;

  /// Create a copy of OcrBoundingGeometry
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $OcrBoundingGeometry_RectangleCopyWith<OcrBoundingGeometry_Rectangle>
      get copyWith => _$OcrBoundingGeometry_RectangleCopyWithImpl<
          OcrBoundingGeometry_Rectangle>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is OcrBoundingGeometry_Rectangle &&
            (identical(other.left, left) || other.left == left) &&
            (identical(other.top, top) || other.top == top) &&
            (identical(other.width, width) || other.width == width) &&
            (identical(other.height, height) || other.height == height));
  }

  @override
  int get hashCode => Object.hash(runtimeType, left, top, width, height);

  @override
  String toString() {
    return 'OcrBoundingGeometry.rectangle(left: $left, top: $top, width: $width, height: $height)';
  }
}

/// @nodoc
abstract mixin class $OcrBoundingGeometry_RectangleCopyWith<$Res>
    implements $OcrBoundingGeometryCopyWith<$Res> {
  factory $OcrBoundingGeometry_RectangleCopyWith(
          OcrBoundingGeometry_Rectangle value,
          $Res Function(OcrBoundingGeometry_Rectangle) _then) =
      _$OcrBoundingGeometry_RectangleCopyWithImpl;
  @useResult
  $Res call(
      {PlatformInt64 left,
      PlatformInt64 top,
      PlatformInt64 width,
      PlatformInt64 height});
}

/// @nodoc
class _$OcrBoundingGeometry_RectangleCopyWithImpl<$Res>
    implements $OcrBoundingGeometry_RectangleCopyWith<$Res> {
  _$OcrBoundingGeometry_RectangleCopyWithImpl(this._self, this._then);

  final OcrBoundingGeometry_Rectangle _self;
  final $Res Function(OcrBoundingGeometry_Rectangle) _then;

  /// Create a copy of OcrBoundingGeometry
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? left = null,
    Object? top = null,
    Object? width = null,
    Object? height = null,
  }) {
    return _then(OcrBoundingGeometry_Rectangle(
      left: null == left
          ? _self.left
          : left // ignore: cast_nullable_to_non_nullable
              as PlatformInt64,
      top: null == top
          ? _self.top
          : top // ignore: cast_nullable_to_non_nullable
              as PlatformInt64,
      width: null == width
          ? _self.width
          : width // ignore: cast_nullable_to_non_nullable
              as PlatformInt64,
      height: null == height
          ? _self.height
          : height // ignore: cast_nullable_to_non_nullable
              as PlatformInt64,
    ));
  }
}

/// @nodoc

class OcrBoundingGeometry_Quadrilateral extends OcrBoundingGeometry {
  const OcrBoundingGeometry_Quadrilateral({required this.points}) : super._();

  final String points;

  /// Create a copy of OcrBoundingGeometry
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $OcrBoundingGeometry_QuadrilateralCopyWith<OcrBoundingGeometry_Quadrilateral>
      get copyWith => _$OcrBoundingGeometry_QuadrilateralCopyWithImpl<
          OcrBoundingGeometry_Quadrilateral>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is OcrBoundingGeometry_Quadrilateral &&
            (identical(other.points, points) || other.points == points));
  }

  @override
  int get hashCode => Object.hash(runtimeType, points);

  @override
  String toString() {
    return 'OcrBoundingGeometry.quadrilateral(points: $points)';
  }
}

/// @nodoc
abstract mixin class $OcrBoundingGeometry_QuadrilateralCopyWith<$Res>
    implements $OcrBoundingGeometryCopyWith<$Res> {
  factory $OcrBoundingGeometry_QuadrilateralCopyWith(
          OcrBoundingGeometry_Quadrilateral value,
          $Res Function(OcrBoundingGeometry_Quadrilateral) _then) =
      _$OcrBoundingGeometry_QuadrilateralCopyWithImpl;
  @useResult
  $Res call({String points});
}

/// @nodoc
class _$OcrBoundingGeometry_QuadrilateralCopyWithImpl<$Res>
    implements $OcrBoundingGeometry_QuadrilateralCopyWith<$Res> {
  _$OcrBoundingGeometry_QuadrilateralCopyWithImpl(this._self, this._then);

  final OcrBoundingGeometry_Quadrilateral _self;
  final $Res Function(OcrBoundingGeometry_Quadrilateral) _then;

  /// Create a copy of OcrBoundingGeometry
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? points = null,
  }) {
    return _then(OcrBoundingGeometry_Quadrilateral(
      points: null == points
          ? _self.points
          : points // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc
mixin _$OutputFormat {
  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is OutputFormat);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'OutputFormat()';
  }
}

/// @nodoc
class $OutputFormatCopyWith<$Res> {
  $OutputFormatCopyWith(OutputFormat _, $Res Function(OutputFormat) __);
}

/// Adds pattern-matching-related methods to [OutputFormat].
extension OutputFormatPatterns on OutputFormat {
  /// A variant of `map` that fallback to returning `orElse`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputFormat_Plain value)? plain,
    TResult Function(OutputFormat_Markdown value)? markdown,
    TResult Function(OutputFormat_Djot value)? djot,
    TResult Function(OutputFormat_Html value)? html,
    TResult Function(OutputFormat_Json value)? json,
    TResult Function(OutputFormat_Structured value)? structured,
    TResult Function(OutputFormat_Custom value)? custom,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case OutputFormat_Plain() when plain != null:
        return plain(_that);
      case OutputFormat_Markdown() when markdown != null:
        return markdown(_that);
      case OutputFormat_Djot() when djot != null:
        return djot(_that);
      case OutputFormat_Html() when html != null:
        return html(_that);
      case OutputFormat_Json() when json != null:
        return json(_that);
      case OutputFormat_Structured() when structured != null:
        return structured(_that);
      case OutputFormat_Custom() when custom != null:
        return custom(_that);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// Callbacks receives the raw object, upcasted.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case final Subclass2 value:
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputFormat_Plain value) plain,
    required TResult Function(OutputFormat_Markdown value) markdown,
    required TResult Function(OutputFormat_Djot value) djot,
    required TResult Function(OutputFormat_Html value) html,
    required TResult Function(OutputFormat_Json value) json,
    required TResult Function(OutputFormat_Structured value) structured,
    required TResult Function(OutputFormat_Custom value) custom,
  }) {
    final _that = this;
    switch (_that) {
      case OutputFormat_Plain():
        return plain(_that);
      case OutputFormat_Markdown():
        return markdown(_that);
      case OutputFormat_Djot():
        return djot(_that);
      case OutputFormat_Html():
        return html(_that);
      case OutputFormat_Json():
        return json(_that);
      case OutputFormat_Structured():
        return structured(_that);
      case OutputFormat_Custom():
        return custom(_that);
    }
  }

  /// A variant of `map` that fallback to returning `null`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputFormat_Plain value)? plain,
    TResult? Function(OutputFormat_Markdown value)? markdown,
    TResult? Function(OutputFormat_Djot value)? djot,
    TResult? Function(OutputFormat_Html value)? html,
    TResult? Function(OutputFormat_Json value)? json,
    TResult? Function(OutputFormat_Structured value)? structured,
    TResult? Function(OutputFormat_Custom value)? custom,
  }) {
    final _that = this;
    switch (_that) {
      case OutputFormat_Plain() when plain != null:
        return plain(_that);
      case OutputFormat_Markdown() when markdown != null:
        return markdown(_that);
      case OutputFormat_Djot() when djot != null:
        return djot(_that);
      case OutputFormat_Html() when html != null:
        return html(_that);
      case OutputFormat_Json() when json != null:
        return json(_that);
      case OutputFormat_Structured() when structured != null:
        return structured(_that);
      case OutputFormat_Custom() when custom != null:
        return custom(_that);
      case _:
        return null;
    }
  }

  /// A variant of `when` that fallback to an `orElse` callback.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? plain,
    TResult Function()? markdown,
    TResult Function()? djot,
    TResult Function()? html,
    TResult Function()? json,
    TResult Function()? structured,
    TResult Function(String field0)? custom,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case OutputFormat_Plain() when plain != null:
        return plain();
      case OutputFormat_Markdown() when markdown != null:
        return markdown();
      case OutputFormat_Djot() when djot != null:
        return djot();
      case OutputFormat_Html() when html != null:
        return html();
      case OutputFormat_Json() when json != null:
        return json();
      case OutputFormat_Structured() when structured != null:
        return structured();
      case OutputFormat_Custom() when custom != null:
        return custom(_that.field0);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// As opposed to `map`, this offers destructuring.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case Subclass2(:final field2):
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() plain,
    required TResult Function() markdown,
    required TResult Function() djot,
    required TResult Function() html,
    required TResult Function() json,
    required TResult Function() structured,
    required TResult Function(String field0) custom,
  }) {
    final _that = this;
    switch (_that) {
      case OutputFormat_Plain():
        return plain();
      case OutputFormat_Markdown():
        return markdown();
      case OutputFormat_Djot():
        return djot();
      case OutputFormat_Html():
        return html();
      case OutputFormat_Json():
        return json();
      case OutputFormat_Structured():
        return structured();
      case OutputFormat_Custom():
        return custom(_that.field0);
    }
  }

  /// A variant of `when` that fallback to returning `null`
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? plain,
    TResult? Function()? markdown,
    TResult? Function()? djot,
    TResult? Function()? html,
    TResult? Function()? json,
    TResult? Function()? structured,
    TResult? Function(String field0)? custom,
  }) {
    final _that = this;
    switch (_that) {
      case OutputFormat_Plain() when plain != null:
        return plain();
      case OutputFormat_Markdown() when markdown != null:
        return markdown();
      case OutputFormat_Djot() when djot != null:
        return djot();
      case OutputFormat_Html() when html != null:
        return html();
      case OutputFormat_Json() when json != null:
        return json();
      case OutputFormat_Structured() when structured != null:
        return structured();
      case OutputFormat_Custom() when custom != null:
        return custom(_that.field0);
      case _:
        return null;
    }
  }
}

/// @nodoc

class OutputFormat_Plain extends OutputFormat {
  const OutputFormat_Plain() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is OutputFormat_Plain);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'OutputFormat.plain()';
  }
}

/// @nodoc

class OutputFormat_Markdown extends OutputFormat {
  const OutputFormat_Markdown() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is OutputFormat_Markdown);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'OutputFormat.markdown()';
  }
}

/// @nodoc

class OutputFormat_Djot extends OutputFormat {
  const OutputFormat_Djot() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is OutputFormat_Djot);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'OutputFormat.djot()';
  }
}

/// @nodoc

class OutputFormat_Html extends OutputFormat {
  const OutputFormat_Html() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is OutputFormat_Html);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'OutputFormat.html()';
  }
}

/// @nodoc

class OutputFormat_Json extends OutputFormat {
  const OutputFormat_Json() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is OutputFormat_Json);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'OutputFormat.json()';
  }
}

/// @nodoc

class OutputFormat_Structured extends OutputFormat {
  const OutputFormat_Structured() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is OutputFormat_Structured);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'OutputFormat.structured()';
  }
}

/// @nodoc

class OutputFormat_Custom extends OutputFormat {
  const OutputFormat_Custom({required this.field0}) : super._();

  final String field0;

  /// Create a copy of OutputFormat
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $OutputFormat_CustomCopyWith<OutputFormat_Custom> get copyWith =>
      _$OutputFormat_CustomCopyWithImpl<OutputFormat_Custom>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is OutputFormat_Custom &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'OutputFormat.custom(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $OutputFormat_CustomCopyWith<$Res>
    implements $OutputFormatCopyWith<$Res> {
  factory $OutputFormat_CustomCopyWith(
          OutputFormat_Custom value, $Res Function(OutputFormat_Custom) _then) =
      _$OutputFormat_CustomCopyWithImpl;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class _$OutputFormat_CustomCopyWithImpl<$Res>
    implements $OutputFormat_CustomCopyWith<$Res> {
  _$OutputFormat_CustomCopyWithImpl(this._self, this._then);

  final OutputFormat_Custom _self;
  final $Res Function(OutputFormat_Custom) _then;

  /// Create a copy of OutputFormat
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(OutputFormat_Custom(
      field0: null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

// dart format on
