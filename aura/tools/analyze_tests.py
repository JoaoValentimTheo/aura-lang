#!/usr/bin/env python3
"""
Test Quality Analysis Tool
Verifies diversity, uniqueness, and quality of generated test files
"""

import os
import hashlib
from pathlib import Path
from collections import defaultdict, Counter
import json

def get_file_hash(filepath):
    """Get MD5 hash of file content"""
    with open(filepath, 'rb') as f:
        return hashlib.md5(f.read()).hexdigest()

def analyze_tests(test_dir="tests"):
    """Analyze test files for quality metrics"""
    root = Path(test_dir)
    
    categories = {
        'oop_tests': root / 'oop_tests',
        'secure_tests': root / 'secure_aura_tests',
        'scope_tests': root / 'syntax_scope_tests',
        'collection_tests': root / 'collections_tests'
    }
    
    results = {}
    all_hashes = defaultdict(list)
    
    for cat_name, cat_dir in categories.items():
        if not cat_dir.exists():
            print(f"⚠️  Directory not found: {cat_dir}")
            continue
        
        files = list(cat_dir.glob('*.aura'))
        file_hashes = {}
        file_sizes = {}
        line_counts = {}
        
        for filepath in files:
            # Get hash
            file_hash = get_file_hash(filepath)
            file_hashes[filepath.name] = file_hash
            all_hashes[file_hash].append(filepath.name)
            
            # Get size
            file_sizes[filepath.name] = filepath.stat().st_size
            
            # Count lines
            with open(filepath, 'r') as f:
                line_counts[filepath.name] = len(f.readlines())
        
        # Analyze
        unique_hashes = len(set(file_hashes.values()))
        duplicates = sum(1 for h in all_hashes.values() if len(h) > 1)
        
        sizes = list(file_sizes.values())
        lines = list(line_counts.values())
        
        results[cat_name] = {
            'total_files': len(files),
            'unique_files': unique_hashes,
            'duplicates': sum(len(f) - 1 for f in all_hashes.values() if len(f) > 1),
            'avg_size_bytes': sum(sizes) / len(sizes) if sizes else 0,
            'min_size_bytes': min(sizes) if sizes else 0,
            'max_size_bytes': max(sizes) if sizes else 0,
            'avg_lines': sum(lines) / len(lines) if lines else 0,
            'min_lines': min(lines) if lines else 0,
            'max_lines': max(lines) if lines else 0,
        }
    
    return results, all_hashes

def print_analysis(results, all_hashes):
    """Print analysis results"""
    print("\n" + "="*70)
    print("TEST QUALITY ANALYSIS REPORT")
    print("="*70 + "\n")
    
    total_files = 0
    total_unique = 0
    total_duplicates = 0
    
    for cat_name, metrics in results.items():
        print(f"\n📊 Category: {cat_name.upper()}")
        print("-" * 70)
        print(f"  Total Files:        {metrics['total_files']:>6}")
        print(f"  Unique Files:       {metrics['unique_files']:>6}")
        print(f"  Duplicate Files:    {metrics['duplicates']:>6}")
        print(f"  Uniqueness:         {metrics['unique_files']/max(metrics['total_files'], 1)*100:>5.1f}%")
        print(f"\n  File Size Stats:")
        print(f"    Average:          {metrics['avg_size_bytes']:>6.0f} bytes")
        print(f"    Min:              {metrics['min_size_bytes']:>6} bytes")
        print(f"    Max:              {metrics['max_size_bytes']:>6} bytes")
        print(f"\n  Line Count Stats:")
        print(f"    Average:          {metrics['avg_lines']:>6.1f} lines")
        print(f"    Min:              {metrics['min_lines']:>6} lines")
        print(f"    Max:              {metrics['max_lines']:>6} lines")
        
        total_files += metrics['total_files']
        total_unique += metrics['unique_files']
        total_duplicates += metrics['duplicates']
    
    print("\n" + "="*70)
    print("OVERALL STATISTICS")
    print("="*70)
    print(f"Total Files:        {total_files:>6}")
    print(f"Unique Files:       {total_unique:>6}")
    print(f"Duplicate Files:    {total_duplicates:>6}")
    print(f"Uniqueness Rate:    {total_unique/max(total_files, 1)*100:>5.1f}%")
    
    # Check for duplicates
    if total_duplicates > 0:
        print(f"\n⚠️  WARNING: Found {total_duplicates} duplicate files!")
        print("\nDuplicate Groups:")
        for file_hash, files in all_hashes.items():
            if len(files) > 1:
                print(f"  {file_hash[:8]}... : {files}")
    else:
        print(f"\n✅ EXCELLENT: All {total_files} files are unique!")
    
    print("\n" + "="*70 + "\n")

def generate_patterns_report(test_dir="tests"):
    """Generate report on pattern distribution"""
    root = Path(test_dir)
    
    pattern_counter = defaultdict(int)
    
    # OOP patterns
    for f in (root / 'oop_tests').glob('*.aura'):
        with open(f, 'r') as file:
            content = file.read()
            if 'Inheritance' in content:
                if 'Deep' in content:
                    pattern_counter['oop:deep_inheritance'] += 1
                else:
                    pattern_counter['oop:simple_inheritance'] += 1
            elif 'Composition' in content:
                pattern_counter['oop:composition'] += 1
            elif 'Factory' in content:
                pattern_counter['oop:factory'] += 1
            elif 'Builder' in content:
                pattern_counter['oop:builder'] += 1
            elif 'Polymorphism' in content:
                pattern_counter['oop:polymorphism'] += 1
            elif 'Delegation' in content:
                pattern_counter['oop:delegation'] += 1
            else:
                pattern_counter['oop:other'] += 1
    
    print("\n📈 PATTERN DISTRIBUTION")
    print("="*70)
    
    for pattern, count in sorted(pattern_counter.items(), key=lambda x: x[1], reverse=True):
        pct = (count / sum(pattern_counter.values()) * 100)
        bar_width = int(pct / 2)
        bar = '█' * bar_width
        print(f"  {pattern:30} {count:5d} ({pct:5.1f}%) {bar}")
    
    print("="*70 + "\n")

if __name__ == "__main__":
    print("🔍 Analyzing test files...\n")
    
    results, all_hashes = analyze_tests()
    print_analysis(results, all_hashes)
    
    # Also generate patterns report
    try:
        generate_patterns_report()
    except Exception as e:
        print(f"⚠️  Could not generate patterns report: {e}\n")
    
    # Save results to JSON
    summary = {
        'timestamp': str(Path('tests').stat().st_mtime),
        'categories': results
    }
    
    with open('tests/ANALYSIS_RESULTS.json', 'w') as f:
        json.dump(summary, f, indent=2)
    
    print("✅ Analysis complete! Results saved to tests/ANALYSIS_RESULTS.json")
