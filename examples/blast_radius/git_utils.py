from unidiff import PatchSet
import io
from typing import Dict, List, Tuple
import logging
import git


def parse_diff(
    repo_path: str, commit_sha_1: str, commit_sha_2: str
) -> Tuple[Dict[str, List[int]], str]:
    repo = git.Repo(repo_path)
    diff_cmd = ["git", "diff", commit_sha_1, commit_sha_2]
    diff_text = repo.git.execute(diff_cmd, with_extended_output=False)
    patch = PatchSet(io.StringIO(diff_text))
    affected_lines = {}
    for patched_file in patch:
        for hunk in patched_file:
            for line in hunk:
                if line.is_added:
                    affected_lines[patched_file.path] = affected_lines.get(
                        patched_file.path, set()
                    ) | {line.target_line_no}
                elif line.is_removed:
                    affected_lines[patched_file.path] = affected_lines.get(
                        patched_file.path, set()
                    ) | {line.source_line_no}
    return affected_lines, diff_text


def clone_repo(repo_url, temp_dir):
    logging.info(f"Cloning repository {repo_url} into {temp_dir}")
    git.Repo.clone_from(repo_url, temp_dir)


def checkout_commit(repo_path, commit):
    logging.info(f"Checking out commit {commit} in {repo_path}")
    repo = git.Repo(repo_path)
    repo.git.checkout(commit)
