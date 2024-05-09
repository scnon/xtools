#[allow(dead_code)]
pub static DART_TMPL: &str = r#"
////////////////////////////////////////////////////////////
////           ____             __ 
////          / __ \____ ______/ /_
////         / / / / __ `/ ___/ __/
////        / /_/ / /_/ / /  / /_  
////       /_____/\__,_/_/   \__/  
////                             
////   GENERATED CODE - DO NOT MODIFY BY HAND
//// 
////////////////////////////////////////////////////////////
library;

{imports}

class {className} {
  {fields}

  {className}({{ctor}});

  factory {className}.fromJson(Map<String, dynamic> json) {
    return {className}(
      {fromJson}
    );
  }

  Map<String, dynamic> toJson() {
    return {
      {toJson}
    };
  }
}
"#;