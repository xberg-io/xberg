interface SqliteDatabase {
  exec(options: string | {
    sql: string;
    bind?: unknown[];
    rowMode?: "object";
    returnValue?: "resultRows";
  }): unknown;
  selectValue(sql: string, bind?: unknown[]): unknown;
  close(): void;
}

interface SqliteModule {
  oo1: {
    DB: new (filename: string, flags?: string) => SqliteDatabase;
    OpfsDb?: new (filename: string) => SqliteDatabase;
  };
}

export default function sqlite3InitModule(options?: {
  locateFile?: (filename: string) => string;
}): Promise<SqliteModule>;
