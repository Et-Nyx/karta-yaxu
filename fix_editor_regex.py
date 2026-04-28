import sys
import glob
import re

def process():
    files = glob.glob('crates/backend/src/editor/*.rs')
    for f_path in files:
        if f_path.endswith('mod.rs') or f_path.endswith('index.rs'):
            continue
            
        with open(f_path, 'r') as f:
            content = f.read()
            
        # Replace .await.unwrap_or_default() with .await?
        content = re.sub(r'\.await\s*\.\s*unwrap_or_default\(\)', r'.await?', content)
        
        # Make sure return type is Result
        if '-> Result<impl IntoResponse, crate::error::AppError>' not in content:
            content = re.sub(r'-> impl IntoResponse\s*\{', r'-> Result<impl IntoResponse, crate::error::AppError> {', content)
        
        # Ensure return Ok()
        # Find instances of render(...) without Ok() or .into_response()
        content = re.sub(r'(?<!Ok\()render\(([^)]+)\)(?!\.into_response\(\))', r'Ok(render(\1).into_response())', content)
        content = re.sub(r'(?<!Ok\()(Redirect::to\([^)]+\))(?!\.into_response\(\))', r'Ok(\1.into_response())', content)
        
        # Replace Html(...) with Ok(Html(...).into_response())
        # First remove any existing into_response() calls for Html
        content = re.sub(r'Html\((.*?)\)\.into_response\(\)', r'Html(\1)', content)
        content = re.sub(r'(?<!Ok\()Html\((.*?)\)', r'Ok(Html(\1).into_response())', content)

        # Remove duplicate Ok(Ok(...)) just in case
        content = content.replace("Ok(Ok(", "Ok(")
        content = content.replace(").into_response())).into_response())", ").into_response())")

        with open(f_path, 'w') as f:
            f.write(content)

process()
