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
            
        # Change signature to return Result
        content = re.sub(r'-> impl IntoResponse \{', r'-> Result<impl IntoResponse, crate::error::AppError> {', content)
        
        # Replace .unwrap_or_default() with ?
        content = content.replace(".fetch_all(&pool).await.unwrap_or_default()", ".fetch_all(&pool).await?")
        content = content.replace(".fetch_all(pool).await.unwrap_or_default()", ".fetch_all(pool).await?")
        content = content.replace(".fetch_one(&pool).await.unwrap_or_default()", ".fetch_one(&pool).await?")
        
        # In editor handlers, they typically return Redirect::to(...).into_response() or render(...).into_response()
        # We need to wrap them in Ok(...)
        content = re.sub(r'(Redirect::to\([^)]+\)\.into_response\(\))', r'Ok(\1)', content)
        content = re.sub(r'(render\([^)]+\)\.into_response\(\))', r'Ok(\1)', content)
        content = re.sub(r'Html\(([^)]+)\)\.into_response\(\)', r'Ok(Html(\1).into_response())', content)
        
        with open(f_path, 'w') as f:
            f.write(content)

process()
