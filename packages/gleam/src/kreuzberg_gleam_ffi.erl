%% Gleam FFI shim — bridges Gleam typed structs to the Elixir Rustler NIF.
%%
%% The Gleam binding generates @external calls to this module instead of
%% directly to Elixir.Kreuzberg.Native. This shim:
%%   1. Converts Gleam ExtractionConfig struct (Erlang tagged tuple) to the
%%      JSON string the NIF expects.
%%   2. Converts the NIF's Erlang map results back to Gleam-typed tuples
%%      that Gleam's record field access (element/2) can handle.
%%
%% NOTE: This file is part of the generated Gleam package — DO NOT EDIT.
%% It is maintained alongside packages/gleam/src/kreuzberg.gleam.

-module(kreuzberg_gleam_ffi).
-compile({no_auto_import, [map_get/2]}).
-export([
    extract_file/3,
    extract_file_sync/3,
    extract_bytes/3,
    extract_bytes_sync/3,
    batch_extract_files_sync/2,
    batch_extract_bytes_sync/2,
    batch_extract_files/2,
    batch_extract_bytes/2,
    detect_mime_type_from_bytes/1,
    get_extensions_for_mime/1,
    list_document_extractors/0,
    list_ocr_backends/0,
    clear_ocr_backends/0,
    list_post_processors/0,
    clear_post_processors/0,
    list_validators/0,
    clear_validators/0,
    embed_texts_async/2,
    render_pdf_page_to_png/4,
    detect_mime_type/2,
    embed_texts/2,
    get_embedding_preset/1,
    list_embedding_presets/0,
    register_ocr_backend/2,
    ocr_backend_process_image_response/2,
    ocr_backend_process_image_file_response/2,
    ocr_backend_supports_language_response/2,
    ocr_backend_backend_type_response/2,
    ocr_backend_supported_languages_response/2,
    ocr_backend_supports_table_detection_response/2,
    ocr_backend_supports_document_processing_response/2,
    ocr_backend_process_document_response/2,
    complete_trait_call/2,
    fail_trait_call/2,
    register_post_processor/2,
    post_processor_process_response/2,
    post_processor_processing_stage_response/2,
    post_processor_should_process_response/2,
    post_processor_estimated_duration_ms_response/2,
    post_processor_priority_response/2,
    register_validator/2,
    validator_validate_response/2,
    validator_should_validate_response/2,
    validator_priority_response/2,
    register_embedding_backend/2,
    embedding_backend_dimensions_response/2,
    embedding_backend_embed_response/2
]).

%% ---------------------------------------------------------------------------
%% Option type conversion helpers
%% ---------------------------------------------------------------------------

%% Convert a Gleam optional string to the Rustler Option<String> Erlang representation.
%% Rustler encodes Option<T> as nil (None) or the bare value (Some(v)).
%% Gleam encodes Option(T) as none (None) or {some, v} (Some(v)).
gleam_opt_str_to_nif(none) -> nil;
gleam_opt_str_to_nif({some, B}) when is_binary(B) -> B;
gleam_opt_str_to_nif(_) -> nil.

%% Convert a Gleam optional int to the Rustler Option<i64> Erlang representation.
gleam_opt_int_to_nif(none) -> nil;
gleam_opt_int_to_nif({some, N}) when is_integer(N) -> N;
gleam_opt_int_to_nif(_) -> nil.

%% ---------------------------------------------------------------------------
%% ExtractionConfig conversion: Gleam tuple → JSON string for NIF
%% ---------------------------------------------------------------------------

%% Convert output_format enum to JSON string.
output_format_to_json(plain) -> <<"plain">>;
output_format_to_json(output_format_markdown) -> <<"markdown">>;
output_format_to_json(djot) -> <<"djot">>;
output_format_to_json(output_format_html) -> <<"html">>;
output_format_to_json(json) -> <<"json">>;
output_format_to_json(structured) -> <<"structured">>;
output_format_to_json({output_format_custom, S}) when is_binary(S) -> S;
output_format_to_json(_) -> <<"plain">>.

%% Convert a Gleam boolean to JSON binary.
bool_to_json(true) -> <<"true">>;
bool_to_json(false) -> <<"false">>.

%% Convert an integer option to a JSON fragment (key:value or empty).
opt_int_to_json_field(_Key, none) -> <<>>;
opt_int_to_json_field(Key, {some, N}) when is_integer(N) ->
    <<",\"", Key/binary, "\":", (integer_to_binary(N))/binary>>.

%% Convert a binary option to a JSON fragment.
opt_str_to_json_field(_Key, none) -> <<>>;
opt_str_to_json_field(Key, {some, B}) when is_binary(B) ->
    Escaped = binary:replace(B, <<"\"">>, <<"\\\"">>, [global]),
    <<",\"", Key/binary, "\":\"", Escaped/binary, "\"">>.

%% Convert SecurityLimits struct to JSON fragment.
security_limits_to_json(none) -> <<>>;
security_limits_to_json({some, {security_limits, MaxArchiveSize, MaxCompressionRatio,
                                MaxFilesInArchive, MaxNestingDepth, MaxEntityLength,
                                MaxContentSize, MaxIterations, MaxXmlDepth, MaxTableCells}}) ->
    <<",\"security_limits\":{",
      "\"max_archive_size\":", (integer_to_binary(MaxArchiveSize))/binary,
      ",\"max_compression_ratio\":", (integer_to_binary(MaxCompressionRatio))/binary,
      ",\"max_files_in_archive\":", (integer_to_binary(MaxFilesInArchive))/binary,
      ",\"max_nesting_depth\":", (integer_to_binary(MaxNestingDepth))/binary,
      ",\"max_entity_length\":", (integer_to_binary(MaxEntityLength))/binary,
      ",\"max_content_size\":", (integer_to_binary(MaxContentSize))/binary,
      ",\"max_iterations\":", (integer_to_binary(MaxIterations))/binary,
      ",\"max_xml_depth\":", (integer_to_binary(MaxXmlDepth))/binary,
      ",\"max_table_cells\":", (integer_to_binary(MaxTableCells))/binary,
      "}">>;
security_limits_to_json(_) -> <<>>.

%% Convert ExtractionConfig Gleam tuple to a JSON binary string for the NIF.
%%
%% extraction_config tuple field order (positions 2..34, 1-indexed):
%%  1=extraction_config tag, 2=use_cache, 3=enable_quality_processing,
%%  4=ocr, 5=force_ocr, 6=force_ocr_pages, 7=disable_ocr,
%%  8=chunking, 9=content_filter, 10=images, 11=pdf_options,
%%  12=token_reduction, 13=language_detection, 14=pages, 15=keywords,
%%  16=postprocessor, 17=html_options, 18=html_output,
%%  19=extraction_timeout_secs, 20=max_concurrent_extractions,
%%  21=result_format, 22=security_limits, 23=output_format,
%%  24=layout, 25=include_document_structure, 26=acceleration,
%%  27=cache_namespace, 28=cache_ttl_secs, 29=email, 30=concurrency,
%%  31=max_archive_depth, 32=tree_sitter, 33=structured_extraction,
%%  34=cancel_token
extraction_config_to_json({extraction_config,
    UseCache, EnableQuality, _Ocr, ForceOcr, _ForceOcrPages, DisableOcr,
    _Chunking, _ContentFilter, _Images, _PdfOptions,
    _TokenReduction, _LanguageDetection, _Pages, _Keywords,
    _Postprocessor, HtmlOptions, _HtmlOutput,
    ExtractionTimeoutSecs, MaxConcurrentExtractions,
    _ResultFormat, SecurityLimits, OutputFormat,
    _Layout, IncludeDocumentStructure, _Acceleration,
    CacheNamespace, CacheTtlSecs, _Email, _Concurrency,
    MaxArchiveDepth, _TreeSitter, _StructuredExtraction, _CancelToken}) ->
    OutputFormatJson = output_format_to_json(OutputFormat),
    SecurityLimitsJson = security_limits_to_json(SecurityLimits),
    TimeoutJson = opt_int_to_json_field(<<"extraction_timeout_secs">>, ExtractionTimeoutSecs),
    MaxConcurrentJson = opt_int_to_json_field(<<"max_concurrent_extractions">>, MaxConcurrentExtractions),
    HtmlOptionsJson = opt_str_to_json_field(<<"html_options">>, HtmlOptions),
    CacheNamespaceJson = opt_str_to_json_field(<<"cache_namespace">>, CacheNamespace),
    CacheTtlSecsJson = opt_int_to_json_field(<<"cache_ttl_secs">>, CacheTtlSecs),
    <<"{",
      "\"use_cache\":", (bool_to_json(UseCache))/binary,
      ",\"enable_quality_processing\":", (bool_to_json(EnableQuality))/binary,
      ",\"force_ocr\":", (bool_to_json(ForceOcr))/binary,
      ",\"disable_ocr\":", (bool_to_json(DisableOcr))/binary,
      ",\"include_document_structure\":", (bool_to_json(IncludeDocumentStructure))/binary,
      ",\"max_archive_depth\":", (integer_to_binary(MaxArchiveDepth))/binary,
      ",\"output_format\":\"", OutputFormatJson/binary, "\"",
      SecurityLimitsJson/binary,
      TimeoutJson/binary,
      MaxConcurrentJson/binary,
      HtmlOptionsJson/binary,
      CacheNamespaceJson/binary,
      CacheTtlSecsJson/binary,
      "}">>;
extraction_config_to_json(_) ->
    <<"{}">>.

%% Convert FileExtractionConfig Gleam tuple to JSON or none.
file_extraction_config_to_json(none) -> none;
file_extraction_config_to_json({some, _}) -> none; %% per-item config not yet supported
file_extraction_config_to_json(_) -> none.

%% Wrap config for NIF: ExtractionConfig → JSON binary.
wrap_config(Config) ->
    extraction_config_to_json(Config).

%% ---------------------------------------------------------------------------
%% Result conversion: NIF Erlang maps → Gleam typed tuples
%% ---------------------------------------------------------------------------

%% Convert a get from a NIF map, defaulting to nil for missing keys.
map_get(Map, Key) when is_map(Map) ->
    maps:get(Key, Map, nil);
map_get(_, _) ->
    nil.

%% Convert nil → none, or value → {some, value}.
opt(nil) -> none;
opt(undefined) -> none;
opt(V) -> {some, V}.

%% Convert nil → none, or apply F and wrap in {some, F(V)}.
opt_map(nil, _F) -> none;
opt_map(undefined, _F) -> none;
opt_map(V, F) -> {some, F(V)}.

%% Convert a FormatMetadata NifStruct (Elixir struct map) to a Gleam format_metadata() tagged tuple.
%%
%% The NIF returns: #{__struct__ => 'Elixir.Kreuzberg.FormatMetadata',
%%                    format_type => <<"excel">>, excel => #{sheet_count => N, ...}, ...}
%% Gleam expects: {excel, {excel_metadata, Option<int>, Option<list<string>>}}
%%                {pdf, binary()} | {docx, docx_metadata()} | etc.
convert_format_metadata(nil) -> none;
convert_format_metadata(undefined) -> none;
convert_format_metadata(M) when is_map(M) ->
    FormatType = map_get(M, format_type),
    Result = case FormatType of
        <<"excel">> ->
            ExcelMap = map_get(M, excel),
            ExcelMeta = if
                is_map(ExcelMap) ->
                    {excel_metadata,
                     opt(map_get(ExcelMap, sheet_count)),
                     opt(map_get(ExcelMap, sheet_names))};
                true ->
                    {excel_metadata, none, none}
            end,
            {some, {excel, ExcelMeta}};
        <<"pdf">> ->
            {some, {pdf, <<>>}};
        <<"docx">> ->
            DocxMap = map_get(M, docx),
            DocxMeta = convert_docx_metadata(DocxMap),
            {some, {docx, DocxMeta}};
        <<"email">> ->
            EmailMap = map_get(M, email),
            EmailMeta = convert_email_metadata(EmailMap),
            {some, {format_metadata_email, EmailMeta}};
        <<"pptx">> ->
            {some, {pptx, {pptx_metadata, none, none, none, none}}};
        <<"archive">> ->
            {some, {archive, {archive_metadata, none, none, [], []}}};
        <<"image">> ->
            {some, {format_metadata_image, <<>>}};
        <<"xml">> ->
            {some, {xml, {xml_metadata, none, none, none, none}}};
        <<"text">> ->
            {some, {format_metadata_text, {text_metadata, none, none, none}}};
        <<"html">> ->
            {some, {format_metadata_html, {html_metadata, none, none, none, none, [], [], none, none, none}}};
        <<"ocr">> ->
            {some, {format_metadata_ocr, {ocr_metadata, none, none, none, none, none, none, []}}};
        <<"csv">> ->
            {some, {csv, {csv_metadata, none, none, none, none}}};
        _ ->
            none
    end,
    Result;
convert_format_metadata(_) -> none.

convert_docx_metadata(nil) ->
    {docx_metadata, none, none, none, none, none, none, none, none, none};
convert_docx_metadata(M) when is_map(M) ->
    {docx_metadata,
     opt(map_get(M, word_count)),
     opt(map_get(M, paragraph_count)),
     opt(map_get(M, page_count)),
     opt(map_get(M, table_count)),
     opt(map_get(M, image_count)),
     opt(map_get(M, has_tracked_changes)),
     opt(map_get(M, revision_count)),
     opt(map_get(M, character_count)),
     opt(map_get(M, line_count))};
convert_docx_metadata(_) ->
    {docx_metadata, none, none, none, none, none, none, none, none, none}.

convert_email_metadata(nil) ->
    {email_metadata, none, none, [], [], [], none, []};
convert_email_metadata(M) when is_map(M) ->
    {email_metadata,
     opt(map_get(M, from_email)),
     opt(map_get(M, from_name)),
     case map_get(M, to_emails) of L when is_list(L) -> L; _ -> [] end,
     case map_get(M, cc_emails) of L when is_list(L) -> L; _ -> [] end,
     case map_get(M, bcc_emails) of L when is_list(L) -> L; _ -> [] end,
     opt(map_get(M, message_id)),
     case map_get(M, attachments) of L when is_list(L) -> L; _ -> [] end};
convert_email_metadata(_) ->
    {email_metadata, none, none, [], [], [], none, []}.

%% Convert a Rustler/Elixir map to a Gleam metadata() tuple.
%%
%% Gleam metadata tuple fields (positions 2-22):
%%   title, subject, authors, keywords, language, created_at, modified_at,
%%   created_by, modified_by, pages, format, image_preprocessing, json_schema,
%%   error, extraction_duration_ms, category, tags, document_version,
%%   abstract_text, output_format, additional
map_to_metadata(nil) ->
    {metadata, none, none, none, none, none, none, none, none, none,
     none, none, none, none, none, none, none, none, none, none, none,
     #{}};
map_to_metadata(M) when is_map(M) ->
    {metadata,
     opt(map_get(M, title)),
     opt(map_get(M, subject)),
     opt(map_get(M, authors)),
     opt(map_get(M, keywords)),
     opt(map_get(M, language)),
     opt(map_get(M, created_at)),
     opt(map_get(M, modified_at)),
     opt(map_get(M, created_by)),
     opt(map_get(M, modified_by)),
     none,  %% pages (PageStructure) — complex, not accessed in tests
     convert_format_metadata(map_get(M, format)),
     none,  %% image_preprocessing
     opt(map_get(M, json_schema)),
     none,  %% error (ErrorMetadata)
     opt(map_get(M, extraction_duration_ms)),
     opt(map_get(M, category)),
     opt(map_get(M, tags)),
     opt(map_get(M, document_version)),
     opt(map_get(M, abstract_text)),
     opt(map_get(M, output_format)),
     maps:get(additional, M, #{})};
map_to_metadata(_) ->
    {metadata, none, none, none, none, none, none, none, none, none,
     none, none, none, none, none, none, none, none, none, none, none,
     #{}}.

%% Convert a Rustler/Elixir NifMap to a Gleam document_structure() tuple.
%%
%% Gleam document_structure tuple: {document_structure, nodes, source_format,
%%   relationships, node_types}
map_to_document_structure(nil) -> none;
map_to_document_structure(undefined) -> none;
map_to_document_structure(M) when is_map(M) ->
    Nodes = map_get(M, nodes),
    NodesList = if is_list(Nodes) -> Nodes; true -> [] end,
    SourceFormat = opt(map_get(M, source_format)),
    Relationships = [],
    NodeTypes = case map_get(M, node_types) of
        L when is_list(L) -> L;
        _ -> []
    end,
    {document_structure, NodesList, SourceFormat, Relationships, NodeTypes};
map_to_document_structure(_) -> none.

%% Convert a NIF ExtractionResult map to a Gleam extraction_result() tuple.
%%
%% Gleam extraction_result tuple fields (positions 2-25):
%%   content, mime_type, metadata, extraction_method, tables,
%%   detected_languages, chunks, images, pages, elements,
%%   djot_content, ocr_elements, document, extracted_keywords,
%%   quality_score, processing_warnings, annotations, children,
%%   uris, structured_output, code_intelligence, llm_usage,
%%   formatted_content, ocr_internal_document
convert_extraction_result({ok, M}) when is_map(M) ->
    {ok, map_to_extraction_result(M)};
convert_extraction_result({error, _} = E) ->
    E;
convert_extraction_result(Other) ->
    Other.

map_to_extraction_result(M) when is_map(M) ->
    {extraction_result,
     map_get(M, content),
     map_get(M, mime_type),
     map_to_metadata(map_get(M, metadata)),
     opt(map_get(M, extraction_method)),
     case map_get(M, tables) of L when is_list(L) -> L; _ -> [] end,
     opt(map_get(M, detected_languages)),
     opt(map_get(M, chunks)),
     opt(map_get(M, images)),
     opt(map_get(M, pages)),
     opt(map_get(M, elements)),
     opt(map_get(M, djot_content)),
     opt(map_get(M, ocr_elements)),
     opt_map(map_get(M, document), fun map_to_document_structure/1),
     opt(map_get(M, extracted_keywords)),
     opt(map_get(M, quality_score)),
     case map_get(M, processing_warnings) of L when is_list(L) -> L; _ -> [] end,
     opt(map_get(M, annotations)),
     opt(map_get(M, children)),
     opt(map_get(M, uris)),
     opt(map_get(M, structured_output)),
     opt(map_get(M, code_intelligence)),
     opt(map_get(M, llm_usage)),
     opt(map_get(M, formatted_content)),
     opt(map_get(M, ocr_internal_document))}.

%% ---------------------------------------------------------------------------
%% Batch items
%% ---------------------------------------------------------------------------

%% Wrap items list for batch file extraction.
wrap_batch_file_items(Items) ->
    [begin
        Path = element(2, Item),
        ConfigJson = file_extraction_config_to_json(element(3, Item)),
        {batch_file_item, Path, ConfigJson}
     end || Item <- Items].

%% Wrap items list for batch bytes extraction.
wrap_batch_bytes_items(Items) ->
    [begin
        Content = element(2, Item),
        MimeType = element(3, Item),
        ConfigJson = file_extraction_config_to_json(element(4, Item)),
        {batch_bytes_item, Content, MimeType, ConfigJson}
     end || Item <- Items].

%% ---------------------------------------------------------------------------
%% Extraction functions (require config conversion + result conversion)
%% ---------------------------------------------------------------------------

%% extract_file / extract_bytes are the Gleam "async" variants.
%% In the Erlang/Elixir runtime there is no native Gleam async, so we route
%% these through the sync NIF which runs on a Tokio thread pool internally.
extract_file(Path, MimeType, Config) ->
    convert_extraction_result(
        'Elixir.Kreuzberg.Native':extract_file_sync(
            Path, gleam_opt_str_to_nif(MimeType), wrap_config(Config))).

extract_file_sync(Path, MimeType, Config) ->
    convert_extraction_result(
        'Elixir.Kreuzberg.Native':extract_file_sync(
            Path, gleam_opt_str_to_nif(MimeType), wrap_config(Config))).

extract_bytes(Content, MimeType, Config) ->
    convert_extraction_result(
        'Elixir.Kreuzberg.Native':extract_bytes_sync(
            Content, MimeType, wrap_config(Config))).

extract_bytes_sync(Content, MimeType, Config) ->
    convert_extraction_result(
        'Elixir.Kreuzberg.Native':extract_bytes_sync(
            Content, MimeType, wrap_config(Config))).

%% Batch extraction: the Elixir NIF does not expose batch_extract_* directly.
%% We implement batch by calling extract_file_sync / extract_bytes_sync per item
%% and returning a list of results (errors are per-item, batch always succeeds).
batch_extract_files_sync(Items, Config) ->
    ConfigJson = wrap_config(Config),
    WrappedItems = wrap_batch_file_items(Items),
    Results = [begin
        Path = element(2, Item),
        convert_extraction_result(
            'Elixir.Kreuzberg.Native':extract_file_sync(Path, nil, ConfigJson))
     end || Item <- WrappedItems],
    {ok, [V || {ok, V} <- Results]}.

batch_extract_bytes_sync(Items, Config) ->
    ConfigJson = wrap_config(Config),
    WrappedItems = wrap_batch_bytes_items(Items),
    Results = [begin
        Content = element(2, Item),
        MimeType = element(3, Item),
        convert_extraction_result(
            'Elixir.Kreuzberg.Native':extract_bytes_sync(Content, MimeType, ConfigJson))
     end || Item <- WrappedItems],
    {ok, [V || {ok, V} <- Results]}.

batch_extract_files(Items, Config) ->
    batch_extract_files_sync(Items, Config).

batch_extract_bytes(Items, Config) ->
    batch_extract_bytes_sync(Items, Config).

%% ---------------------------------------------------------------------------
%% Pass-through functions (no struct conversion needed)
%% ---------------------------------------------------------------------------

detect_mime_type_from_bytes(Content) ->
    'Elixir.Kreuzberg.Native':detect_mime_type_from_bytes(Content).

get_extensions_for_mime(MimeType) ->
    'Elixir.Kreuzberg.Native':get_extensions_for_mime(MimeType).

list_document_extractors() ->
    'Elixir.Kreuzberg.Native':list_document_extractors().

list_ocr_backends() ->
    'Elixir.Kreuzberg.Native':list_ocr_backends().

clear_ocr_backends() ->
    'Elixir.Kreuzberg.Native':clear_ocr_backends().

list_post_processors() ->
    'Elixir.Kreuzberg.Native':list_post_processors().

clear_post_processors() ->
    'Elixir.Kreuzberg.Native':clear_post_processors().

list_validators() ->
    'Elixir.Kreuzberg.Native':list_validators().

clear_validators() ->
    'Elixir.Kreuzberg.Native':clear_validators().

embed_texts_async(Texts, Config) ->
    'Elixir.Kreuzberg.Native':embed_texts_async(Texts, Config).

render_pdf_page_to_png(PdfBytes, PageIndex, Dpi, Password) ->
    'Elixir.Kreuzberg.Native':render_pdf_page_to_png(
        PdfBytes, PageIndex,
        gleam_opt_int_to_nif(Dpi),
        gleam_opt_str_to_nif(Password)).

detect_mime_type(Path, CheckExists) ->
    'Elixir.Kreuzberg.Native':detect_mime_type(Path, CheckExists).

embed_texts(Texts, Config) ->
    'Elixir.Kreuzberg.Native':embed_texts(Texts, Config).

get_embedding_preset(Name) ->
    'Elixir.Kreuzberg.Native':get_embedding_preset(Name).

list_embedding_presets() ->
    'Elixir.Kreuzberg.Native':list_embedding_presets().

register_ocr_backend(Pid, PluginName) ->
    'Elixir.Kreuzberg.Native':register_ocr_backend(Pid, PluginName).

ocr_backend_process_image_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':ocr_backend_process_image_response(CallId, Result).

ocr_backend_process_image_file_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':ocr_backend_process_image_file_response(CallId, Result).

ocr_backend_supports_language_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':ocr_backend_supports_language_response(CallId, Result).

ocr_backend_backend_type_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':ocr_backend_backend_type_response(CallId, Result).

ocr_backend_supported_languages_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':ocr_backend_supported_languages_response(CallId, Result).

ocr_backend_supports_table_detection_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':ocr_backend_supports_table_detection_response(CallId, Result).

ocr_backend_supports_document_processing_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':ocr_backend_supports_document_processing_response(CallId, Result).

ocr_backend_process_document_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':ocr_backend_process_document_response(CallId, Result).

complete_trait_call(ReplyId, ResultJson) ->
    'Elixir.Kreuzberg.Native':complete_trait_call(ReplyId, ResultJson).

fail_trait_call(ReplyId, ErrorMessage) ->
    'Elixir.Kreuzberg.Native':fail_trait_call(ReplyId, ErrorMessage).

register_post_processor(Pid, PluginName) ->
    'Elixir.Kreuzberg.Native':register_post_processor(Pid, PluginName).

post_processor_process_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':post_processor_process_response(CallId, Result).

post_processor_processing_stage_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':post_processor_processing_stage_response(CallId, Result).

post_processor_should_process_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':post_processor_should_process_response(CallId, Result).

post_processor_estimated_duration_ms_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':post_processor_estimated_duration_ms_response(CallId, Result).

post_processor_priority_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':post_processor_priority_response(CallId, Result).

register_validator(Pid, PluginName) ->
    'Elixir.Kreuzberg.Native':register_validator(Pid, PluginName).

validator_validate_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':validator_validate_response(CallId, Result).

validator_should_validate_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':validator_should_validate_response(CallId, Result).

validator_priority_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':validator_priority_response(CallId, Result).

register_embedding_backend(Pid, PluginName) ->
    'Elixir.Kreuzberg.Native':register_embedding_backend(Pid, PluginName).

embedding_backend_dimensions_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':embedding_backend_dimensions_response(CallId, Result).

embedding_backend_embed_response(CallId, Result) ->
    'Elixir.Kreuzberg.Native':embedding_backend_embed_response(CallId, Result).
